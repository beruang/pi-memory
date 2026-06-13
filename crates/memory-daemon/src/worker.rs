use memory_db::{
    AuditRepository, ConflictsRepository, EmbeddingsRepository, EventsRepository,
    EvidenceRepository, ObservationsRepository,
};
use memory_providers::{create_consolidation_provider, create_embedding_provider, ProviderConfig};
use sqlx::PgPool;
use sqlx::Row;
use tracing::{error, info};

use super::queue::{Job, JobQueue, JobType};

pub struct ConsolidationWorker {
    queue: std::sync::Arc<JobQueue>,
    pool: PgPool,
    provider_type: String,
    provider_config: ProviderConfig,
}

impl ConsolidationWorker {
    pub fn new(
        queue: std::sync::Arc<JobQueue>,
        pool: PgPool,
        provider_type: String,
        provider_config: ProviderConfig,
    ) -> Self {
        Self {
            queue,
            pool,
            provider_type,
            provider_config,
        }
    }

    pub async fn run(&self) {
        info!("Consolidation worker started");
        loop {
            if let Some(job) = self.queue.dequeue().await {
                if matches!(job.job_type, JobType::ConsolidateSession) {
                    match self.process_consolidation(&job).await {
                        Ok(_) => info!(job_id = %job.id, "Consolidation job completed"),
                        Err(e) => {
                            error!(job_id = %job.id, error = %e, "Consolidation job failed");
                            self.queue.nack(job).await;
                        }
                    }
                } else {
                    self.queue.enqueue(job).await;
                }
            }
        }
    }

    async fn process_consolidation(&self, job: &Job) -> Result<(), String> {
        let session_id = job.payload["session_id"]
            .as_str()
            .ok_or("missing session_id")?;
        let project_id = job.payload
            .get("project_id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok());

        // Create repos
        let events_repo = EventsRepository::new(self.pool.clone());
        let obs_repo = ObservationsRepository::new(self.pool.clone());
        let evidence_repo = EvidenceRepository::new(self.pool.clone());
        let embeddings_repo = EmbeddingsRepository::new(self.pool.clone());
        let conflicts_repo = ConflictsRepository::new(self.pool.clone());
        let audit_repo = AuditRepository::new(self.pool.clone());

        // Load session events
        let events = events_repo
            .list_by_session(session_id, 200)
            .await
            .map_err(|e| e.to_string())?;

        // Load existing observations for conflict detection
        let existing = obs_repo
            .list_active_with_entities(project_id)
            .await
            .map_err(|e| e.to_string())?;

        // Create consolidation provider
        let provider = create_consolidation_provider(&self.provider_type, &self.provider_config)
            .map_err(|e| e.to_string())?;

        let input = memory_core::ConsolidationInput {
            session_id: session_id.to_string(),
            project_id,
            events: events.clone(),
            existing_observations: existing.clone(),
            user_instructions: None,
        };

        let candidates = provider
            .consolidate(input)
            .await
            .map_err(|e| e.to_string())?;

        if candidates.is_empty() {
            info!(session_id, "No consolidation candidates generated");
            return Ok(());
        }

        let embedding_provider = create_embedding_provider(
            "mock",
            &ProviderConfig {
                api_key: None,
                model: "mock".into(),
                base_url: None,
                dimensions: Some(1536),
            },
        )
        .map_err(|e| e.to_string())?;

        for candidate in &candidates {
            let mut obs = memory_core::Observation::new(
                candidate.scope,
                session_id.to_string(),
                candidate.kind,
                candidate.summary.clone(),
                candidate.confidence,
                candidate.sensitivity,
            )
            .map_err(|e| e.to_string())?;
            obs.project_id = project_id;
            obs.entities = candidate.entities.clone();
            obs.files = candidate.files.clone();
            obs.commands = candidate.commands.clone();

            let conflicts = memory_core::detect_conflicts(&obs, &existing);
            obs.status = if conflicts.is_empty() {
                memory_core::MemoryStatus::Active
            } else {
                memory_core::MemoryStatus::Conflicted
            };

            let _saved = obs_repo
                .insert_with_links(&obs, &candidate.files, &candidate.entities, &candidate.commands, &[])
                .await
                .map_err(|e| e.to_string())?;

            for event_id in &candidate.source_event_ids {
                let mut evidence = memory_core::EvidenceRef::new(
                    obs.id,
                    memory_core::EvidenceSourceType::Message,
                    event_id.to_string(),
                );
                if let Some(ref rationale) = candidate.rationale {
                    evidence = evidence.with_excerpt(rationale.clone());
                }
                evidence_repo
                    .insert(&evidence)
                    .await
                    .map_err(|e| e.to_string())?;
            }

            let embedding_vec = embedding_provider
                .embed(&obs.summary)
                .await
                .map_err(|e| e.to_string())?;
            embeddings_repo
                .upsert_embedding(obs.id, "mock", 1536, &embedding_vec)
                .await
                .map_err(|e| e.to_string())?;

            for conflict in &conflicts {
                let _ = conflicts_repo
                    .insert_conflict(
                        conflict.left_observation_id,
                        conflict.right_observation_id,
                        &conflict.conflict_type.to_string(),
                        &conflict.description,
                    )
                    .await;
            }
        }

        audit_repo
            .insert(
                "consolidation_worker",
                None,
                "consolidate_session",
                None,
                None,
                Some(&serde_json::json!({
                    "session_id": session_id,
                    "events_loaded": events.len(),
                    "candidates_generated": candidates.len(),
                })),
            )
            .await
            .map_err(|e| e.to_string())?;

        info!(
            session_id,
            events = events.len(),
            candidates = candidates.len(),
            "Consolidation complete"
        );
        Ok(())
    }
}

pub struct EmbeddingWorker {
    queue: std::sync::Arc<JobQueue>,
    pool: PgPool,
}

impl EmbeddingWorker {
    pub fn new(queue: std::sync::Arc<JobQueue>, pool: PgPool) -> Self {
        Self { queue, pool }
    }

    pub async fn run(&self) {
        info!("Embedding worker started");
        loop {
            if let Some(job) = self.queue.dequeue().await {
                if matches!(job.job_type, JobType::GenerateEmbedding) {
                    match self.process_embedding(&job).await {
                        Ok(_) => info!(job_id = %job.id, "Embedding job completed"),
                        Err(e) => {
                            error!(job_id = %job.id, error = %e, "Embedding job failed");
                            self.queue.nack(job).await;
                        }
                    }
                } else {
                    self.queue.enqueue(job).await;
                }
            }
        }
    }

    async fn process_embedding(&self, job: &Job) -> Result<(), String> {
        let text = job.payload["text"].as_str().ok_or("missing text")?;
        let observation_id = job.payload["observation_id"]
            .as_str()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .ok_or("missing or invalid observation_id")?;

        let provider = create_embedding_provider(
            "mock",
            &ProviderConfig {
                api_key: None,
                model: "mock".into(),
                base_url: None,
                dimensions: Some(1536),
            },
        )
        .map_err(|e| e.to_string())?;

        let embedding = provider.embed(text).await.map_err(|e| e.to_string())?;

        let embeddings_repo = EmbeddingsRepository::new(self.pool.clone());
        embeddings_repo
            .upsert_embedding(observation_id, "mock", 1536, &embedding)
            .await
            .map_err(|e| e.to_string())?;

        info!(
            observation_id = %observation_id,
            dimensions = embedding.len(),
            "Embedding generated and stored"
        );
        Ok(())
    }
}

pub struct CleanupEventsWorker {
    queue: std::sync::Arc<JobQueue>,
    pool: PgPool,
    retention_days: u32,
}

impl CleanupEventsWorker {
    pub fn new(queue: std::sync::Arc<JobQueue>, pool: PgPool, retention_days: u32) -> Self {
        Self {
            queue,
            pool,
            retention_days,
        }
    }

    pub async fn run(&self) {
        info!(
            "Cleanup events worker started (retention={} days)",
            self.retention_days
        );
        loop {
            if let Some(job) = self.queue.dequeue().await {
                if matches!(job.job_type, JobType::CleanupEvents) {
                    match self.process_cleanup(&job).await {
                        Ok(_) => info!(job_id = %job.id, "Cleanup job completed"),
                        Err(e) => {
                            error!(job_id = %job.id, error = %e, "Cleanup job failed");
                            self.queue.nack(job).await;
                        }
                    }
                } else {
                    self.queue.enqueue(job).await;
                }
            }
        }
    }

    async fn process_cleanup(&self, job: &Job) -> Result<(), String> {
        let _project_id = job.payload.get("project_id").and_then(|v| v.as_str());
        let days = self.retention_days;

        let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(days as i64);
        let events_repo = EventsRepository::new(self.pool.clone());
        let deleted = events_repo
            .delete_older_than(cutoff)
            .await
            .map_err(|e| e.to_string())?;

        info!("Cleaned up {} session_events older than {} days", deleted, days);
        Ok(())
    }
}

pub struct DetectConflictsWorker {
    queue: std::sync::Arc<JobQueue>,
    pool: PgPool,
}

impl DetectConflictsWorker {
    pub fn new(queue: std::sync::Arc<JobQueue>, pool: PgPool) -> Self {
        Self { queue, pool }
    }

    pub async fn run(&self) {
        info!("Detect conflicts worker started");
        loop {
            if let Some(job) = self.queue.dequeue().await {
                if matches!(job.job_type, JobType::DetectConflicts) {
                    match self.process_detection(&job).await {
                        Ok(_) => info!(job_id = %job.id, "Detect conflicts job completed"),
                        Err(e) => {
                            error!(job_id = %job.id, error = %e, "Detect conflicts job failed");
                            self.queue.nack(job).await;
                        }
                    }
                } else {
                    self.queue.enqueue(job).await;
                }
            }
        }
    }

    async fn process_detection(&self, job: &Job) -> Result<(), String> {
        let project_id = job
            .payload
            .get("project_id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok());

        // Load recent active observations with their links
        let rows = if let Some(pid) = project_id {
            sqlx::query(
                r#"
                SELECT o.*,
                    COALESCE(array_agg(DISTINCT f.file_path) FILTER (WHERE f.file_path IS NOT NULL), '{}') as files,
                    COALESCE(array_agg(DISTINCT e.entity) FILTER (WHERE e.entity IS NOT NULL), '{}') as entities
                FROM observations o
                LEFT JOIN observation_files f ON f.observation_id = o.id
                LEFT JOIN observation_entities e ON e.observation_id = o.id
                WHERE o.status = 'active' AND o.project_id = $1
                GROUP BY o.id
                LIMIT 500
                "#,
            )
            .bind(pid)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx::query(
                r#"
                SELECT o.*,
                    COALESCE(array_agg(DISTINCT f.file_path) FILTER (WHERE f.file_path IS NOT NULL), '{}') as files,
                    COALESCE(array_agg(DISTINCT e.entity) FILTER (WHERE e.entity IS NOT NULL), '{}') as entities
                FROM observations o
                LEFT JOIN observation_files f ON f.observation_id = o.id
                LEFT JOIN observation_entities e ON e.observation_id = o.id
                WHERE o.status = 'active'
                GROUP BY o.id
                LIMIT 500
                "#,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        };

        // Build observations from rows (simplified - just detect conflicts on summary/entity overlap)
        for row in &rows {
            let id: uuid::Uuid = row.get("id");
            let kind: memory_core::MemoryKind = row.get("kind");
            let summary: String = row.get("summary");
            let entities_val: Vec<String> = row.try_get("entities").unwrap_or_default();
            let files_val: Vec<String> = row.try_get("files").unwrap_or_default();

            let obs = memory_core::Observation {
                id,
                scope: row.get("scope"),
                project_id: row.get("project_id"),
                user_id: row.get("user_id"),
                organization_id: row.get("organization_id"),
                session_id: row.get("session_id"),
                kind,
                summary,
                entities: entities_val,
                files: files_val,
                commands: vec![],
                links: vec![],
                confidence: row.get("confidence"),
                sensitivity: row.get("sensitivity"),
                status: row.get("status"),
                evidence: vec![],
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_accessed_at: row.get("last_accessed_at"),
                last_confirmed_at: row.get("last_confirmed_at"),
                valid_until: row.get("valid_until"),
                supersedes: vec![],
                superseded_by: row.get("superseded_by"),
                metadata: row.get("metadata"),
            };

            // Compare against all other observations
            for other_row in &rows {
                if other_row.get::<uuid::Uuid, _>("id") == id {
                    continue;
                }
                let other_id: uuid::Uuid = other_row.get("id");
                let other_kind: memory_core::MemoryKind = other_row.get("kind");
                let other_summary: String = other_row.get("summary");
                let other_entities: Vec<String> = other_row.try_get("entities").unwrap_or_default();

                if obs.kind == other_kind && !obs.entities.is_empty() && !other_entities.is_empty()
                {
                    let overlap: Vec<&String> = obs
                        .entities
                        .iter()
                        .filter(|e| other_entities.contains(e))
                        .collect();
                    if !overlap.is_empty() {
                        // Check for contradiction
                        if memory_core::summaries_contradict(&obs.summary, &other_summary) {
                            let conflict_type = match obs.kind {
                                memory_core::MemoryKind::Preference => {
                                    "same_preference_different_preference"
                                }
                                memory_core::MemoryKind::Decision => {
                                    "same_decision_incompatible_decision"
                                }
                                memory_core::MemoryKind::Dependency => {
                                    "same_dependency_different_version"
                                }
                                memory_core::MemoryKind::Policy => {
                                    "same_policy_incompatible_policy"
                                }
                                _ => "same_entity_incompatible_value",
                            };
                            sqlx::query(
                                r#"INSERT INTO observation_conflicts (left_observation_id, right_observation_id, conflict_type, description)
                                VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING"#,
                            )
                            .bind(id)
                            .bind(other_id)
                            .bind(conflict_type)
                            .bind(format!("Conflict on {:?} between '{}' and '{}'", overlap, obs.summary, other_summary))
                            .execute(&self.pool)
                            .await
                            .map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
