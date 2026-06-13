use memory_core::{
    detect_conflicts, scan_for_secrets, strip_private_blocks, valid_transition,
    AccessContext, ConflictRecord, ConsolidationInput, ConsolidationProvider,
    MemoryConfidence, MemoryError, MemoryKind, MemoryScope, MemorySensitivity, MemoryStatus,
    Observation, PrivateBlockRange,
};
use memory_db::{
    AuditRepository, ConflictsRepository, EmbeddingsRepository, EventsRepository,
    EvidenceRepository, ObservationsRepository, SearchParams, SearchRepository,
    SupersessionsRepository,
};
use memory_providers::{create_embedding_provider, ProviderConfig};
use uuid::Uuid;

pub struct MemoryMcpServer {
    obs_repo: ObservationsRepository,
    evidence_repo: EvidenceRepository,
    embeddings_repo: EmbeddingsRepository,
    search_repo: SearchRepository,
    conflicts_repo: ConflictsRepository,
    audit_repo: AuditRepository,
    supers_repo: SupersessionsRepository,
    events_repo: EventsRepository,
    consolidation_provider: Box<dyn ConsolidationProvider>,
    access_context: AccessContext,
}

impl MemoryMcpServer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        obs_repo: ObservationsRepository,
        evidence_repo: EvidenceRepository,
        embeddings_repo: EmbeddingsRepository,
        search_repo: SearchRepository,
        conflicts_repo: ConflictsRepository,
        audit_repo: AuditRepository,
        supers_repo: SupersessionsRepository,
        events_repo: EventsRepository,
        consolidation_provider: Box<dyn ConsolidationProvider>,
        access_context: AccessContext,
    ) -> Self {
        Self {
            obs_repo,
            evidence_repo,
            embeddings_repo,
            search_repo,
            conflicts_repo,
            audit_repo,
            supers_repo,
            events_repo,
            consolidation_provider,
            access_context,
        }
    }

    pub async fn handle_tool_call(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        match tool_name {
            "memory.recall" => self.handle_recall(params).await,
            "memory.search" => self.handle_search(params).await,
            "memory.get" => self.handle_get(params).await,
            "memory.write" => self.handle_write(params).await,
            "memory.update" => self.handle_update(params).await,
            "memory.mark_obsolete" => self.handle_mark_obsolete(params).await,
            "memory.consolidate_session" => self.handle_consolidate(params).await,
            "memory.session_start" => self.handle_session_start(params).await,
            "memory.link_file" => self.handle_link_file(params).await,
            "memory.list_conflicts" => self.handle_list_conflicts(params).await,
            "memory.resolve_conflict" => self.handle_resolve_conflict(params).await,
            "memory.delete" => self.handle_delete(params).await,
            _ => Err(MemoryError::InvalidId(format!("unknown tool: {}", tool_name))),
        }
    }

    async fn handle_recall(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let task = params["task"].as_str().unwrap_or_default().to_string();
        let scope = params["scope"].as_str().unwrap_or("project").to_string();
        let project_id = parse_optional_uuid(&params, "project_id");

        // Early return for empty task — prevents embedding provider crash
        if task.trim().is_empty() {
            return Ok(serde_json::json!({
                "memories": [],
                "token_estimate": 0,
                "budget_exceeded": false,
            }));
        }

        // Authorization: caller must have read access at this scope
        let mem_scope: MemoryScope = scope.parse().unwrap_or(MemoryScope::Project);
        self.access_context.check_read_access(
            &mem_scope,
            project_id,
            None,
            None,
            &MemorySensitivity::Internal,
        )?;
        let kinds: Option<Vec<String>> = params["kinds"].as_array().map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
        let files: Vec<String> = params["files"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let min_confidence = params["min_confidence"].as_str().map(String::from);
        let token_budget = params["token_budget"].as_u64().unwrap_or(1200) as usize;

        let provider = create_embedding_provider(
            "mock",
            &ProviderConfig {
                api_key: None,
                model: "mock".into(),
                base_url: None,
                dimensions: Some(1536),
            },
        )?;
        let query_embedding = provider.embed(&task).await?;

        let results = self
            .search_repo
            .hybrid_search(SearchParams {
                query_embedding,
                text_query: task.clone(),
                scope,
                project_id,
                kinds,
                files: if files.is_empty() { None } else { Some(files) },
                entities: None,
                limit: 10,
                min_confidence,
            })
            .await?;

        let mut memories = Vec::new();
        let mut token_estimate = 0;
        let chars_per_token = 4;

        for scored in &results {
            let item_tokens = scored.observation.summary.len() / chars_per_token;
            if token_estimate + item_tokens > token_budget {
                break;
            }
            token_estimate += item_tokens;
            memories.push(serde_json::json!({
                "id": scored.observation.id.to_string(),
                "kind": scored.observation.kind.to_string(),
                "summary": scored.observation.summary,
                "confidence": scored.observation.confidence.to_string(),
                "status": scored.observation.status.to_string(),
                "evidence_count": scored.observation.evidence.len(),
            }));
        }

        Ok(serde_json::json!({
            "memories": memories,
            "token_estimate": token_estimate,
            "budget_exceeded": token_estimate > token_budget,
        }))
    }

    async fn handle_search(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let query = params["query"].as_str().unwrap_or_default().to_string();
        let scope = params["scope"].as_str().unwrap_or("project").to_string();
        let project_id = parse_optional_uuid(&params, "project_id");

        // Early return for empty query — prevents embedding provider crash
        if query.trim().is_empty() {
            return Ok(serde_json::json!({"results": []}));
        }

        // Authorization: caller must have read access at this scope
        let mem_scope: MemoryScope = scope.parse().unwrap_or(MemoryScope::Project);
        self.access_context.check_read_access(
            &mem_scope,
            project_id,
            None,
            None,
            &MemorySensitivity::Internal,
        )?;
        let kinds: Option<Vec<String>> = params["kinds"].as_array().map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
        let files: Vec<String> = params["files"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let min_confidence = params["min_confidence"].as_str().map(String::from);
        let limit = params["limit"].as_u64().unwrap_or(10) as i64;

        let provider = create_embedding_provider(
            "mock",
            &ProviderConfig {
                api_key: None,
                model: "mock".into(),
                base_url: None,
                dimensions: Some(1536),
            },
        )?;
        let query_embedding = provider.embed(&query).await?;

        let results = self
            .search_repo
            .hybrid_search(SearchParams {
                query_embedding,
                text_query: query,
                scope,
                project_id,
                kinds,
                files: if files.is_empty() { None } else { Some(files) },
                entities: None,
                limit,
                min_confidence,
            })
            .await?;

        Ok(serde_json::json!({
            "results": results.iter().map(|s| serde_json::json!({
                "id": s.observation.id.to_string(),
                "kind": s.observation.kind.to_string(),
                "summary": s.observation.summary,
                "confidence": s.observation.confidence.to_string(),
                "score": s.final_score,
            })).collect::<Vec<_>>()
        }))
    }

    async fn handle_get(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidId("missing id".into()))?;
        let id = Uuid::parse_str(id_str).map_err(|e| MemoryError::InvalidId(e.to_string()))?;

        let obs = self
            .obs_repo
            .get_by_id_with_links(id)
            .await?
            .ok_or(MemoryError::ObservationNotFound(id))?;

        // Authorization: caller must have read access to this observation
        self.access_context.check_read_access(
            &obs.scope,
            obs.project_id,
            obs.user_id,
            obs.organization_id,
            &obs.sensitivity,
        )?;

        let evidence = self.evidence_repo.get_by_observation_id(id).await?;

        Ok(serde_json::json!({
            "id": obs.id.to_string(),
            "scope": obs.scope.to_string(),
            "kind": obs.kind.to_string(),
            "summary": obs.summary,
            "confidence": obs.confidence.to_string(),
            "sensitivity": obs.sensitivity.to_string(),
            "status": obs.status.to_string(),
            "files": obs.files,
            "entities": obs.entities,
            "commands": obs.commands,
            "supersedes": obs.supersedes.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
            "evidence": evidence.iter().map(|e| serde_json::json!({
                "id": e.id.to_string(),
                "source_type": e.source_type.to_string(),
                "source_id": e.source_id,
                "excerpt": e.excerpt,
            })).collect::<Vec<_>>(),
            "evidence_count": evidence.len(),
            "created_at": obs.created_at.to_string(),
            "updated_at": obs.updated_at.to_string(),
        }))
    }

    async fn handle_write(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let actor = params["actor"].as_str().unwrap_or("agent");
        let scope_str = params["scope"].as_str().unwrap_or("project");
        let scope: MemoryScope = scope_str.parse().map_err(|_| MemoryError::InvalidScope)?;
        let session_id = params["session_id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let kind_str = params["kind"].as_str().unwrap_or("fact");
        let kind: MemoryKind = kind_str_to_kind(kind_str);
        let raw_summary = params["summary"].as_str().unwrap_or_default().to_string();
        let confidence_str = params["confidence"].as_str().unwrap_or("medium");
        let confidence = confidence_str_to_confidence(confidence_str);
        let sensitivity_str = params["sensitivity"].as_str().unwrap_or("internal");

        // Authorization: caller must have write access at this scope and sensitivity
        let project_id = parse_optional_uuid(&params, "project_id");
        let user_id = parse_optional_uuid(&params, "user_id");
        let org_id = parse_optional_uuid(&params, "organization_id");
        let write_sensitivity = sensitivity_str_to_sensitivity(sensitivity_str);
        self.access_context.check_write_access(
            &scope,
            project_id,
            user_id,
            org_id,
            &write_sensitivity,
        )?;

        // Step 1: Strip private blocks
        let (clean_summary, redacted_ranges): (String, Vec<PrivateBlockRange>) =
            strip_private_blocks(&raw_summary);
        let had_private_blocks = !redacted_ranges.is_empty();

        // Step 2: Reject if secrets detected
        if !scan_for_secrets(&clean_summary).is_empty() {
            return Err(MemoryError::SecretContentRejected);
        }

        // Step 3: Classify sensitivity (upgrade to Private if private blocks were stripped)
        let mut sensitivity = write_sensitivity;
        if sensitivity != MemorySensitivity::Secret && had_private_blocks {
            sensitivity = MemorySensitivity::Private;
        }

        // Step 4: Build observation
        let mut obs = Observation::new(
            scope,
            session_id,
            kind,
            clean_summary,
            confidence,
            sensitivity,
        )?;
        obs.project_id = parse_optional_uuid(&params, "project_id");
        obs.user_id = parse_optional_uuid(&params, "user_id");
        obs.organization_id = parse_optional_uuid(&params, "organization_id");

        if let Some(arr) = params["entities"].as_array() {
            obs.entities = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(arr) = params["files"].as_array() {
            obs.files = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(arr) = params["commands"].as_array() {
            obs.commands = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(evidence_arr) = params["evidence"].as_array() {
            for ev in evidence_arr {
                let source_type = ev["source_type"].as_str().unwrap_or("manual_entry");
                let source_id = ev["source_id"].as_str().unwrap_or_default().to_string();
                let excerpt = ev["excerpt"].as_str().map(String::from);
                let mut evidence_ref = memory_core::EvidenceRef::new(
                    obs.id,
                    source_type_str_to_enum(source_type),
                    source_id,
                );
                if let Some(ex) = excerpt {
                    evidence_ref = evidence_ref.with_excerpt(ex);
                }
                obs.evidence.push(evidence_ref);
            }
        }

        if obs.evidence.is_empty() {
            return Err(MemoryError::MissingEvidence);
        }

        // Step 5: Conflict detection
        let existing = self
            .obs_repo
            .list_active_with_entities(obs.project_id)
            .await?;
        let conflicts: Vec<ConflictRecord> = detect_conflicts(&obs, &existing);

        // If conflicts detected, mark as conflicted
        let final_status = if !conflicts.is_empty() {
            obs.status = MemoryStatus::Conflicted;
            MemoryStatus::Conflicted
        } else {
            obs.status = MemoryStatus::Active;
            MemoryStatus::Active
        };

        // Step 6: Insert observation with links
        let saved = self
            .obs_repo
            .insert_with_links(
                &obs,
                &obs.files,
                &obs.entities,
                &obs.commands,
                &obs.supersedes,
            )
            .await?;

        // Step 7: Store evidence
        for ev in &obs.evidence {
            self.evidence_repo.insert(ev).await?;
        }

        // Step 8: Write conflict records if any
        let mut conflict_ids: Vec<Uuid> = Vec::new();
        for conflict in &conflicts {
            let result = self
                .conflicts_repo
                .insert_conflict(
                    conflict.left_observation_id,
                    conflict.right_observation_id,
                    &conflict.conflict_type.to_string(),
                    &conflict.description,
                )
                .await;
            if let Ok(record) = result {
                conflict_ids.push(record.id);
            }
        }

        // Step 9: Generate and store embedding
        let provider = create_embedding_provider(
            "mock",
            &ProviderConfig {
                api_key: None,
                model: "mock".into(),
                base_url: None,
                dimensions: Some(1536),
            },
        )?;
        let embedding_vec = provider.embed(&obs.summary).await?;
        self.embeddings_repo
            .upsert_embedding(obs.id, "mock", 1536, &embedding_vec)
            .await?;

        // Step 10: Write audit log
        self.audit_repo
            .insert(
                actor,
                None,
                "write",
                Some(obs.id),
                None,
                Some(&serde_json::json!({
                    "scope": obs.scope.to_string(),
                    "kind": obs.kind.to_string(),
                    "confidence": obs.confidence.to_string(),
                    "sensitivity": obs.sensitivity.to_string(),
                })),
            )
            .await?;

        Ok(serde_json::json!({
            "id": saved.id.to_string(),
            "status": final_status.to_string(),
            "conflicts": conflict_ids.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
            "private_blocks_redacted": redacted_ranges.iter().map(|r| serde_json::json!({
                "start": r.start,
                "end": r.end
            })).collect::<Vec<_>>(),
        }))
    }

    async fn handle_update(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let actor = params["actor"].as_str().unwrap_or("agent");
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidId("missing id".into()))?;
        let id = Uuid::parse_str(id_str).map_err(|e| MemoryError::InvalidId(e.to_string()))?;

        // Validate ALL input fields BEFORE database lookup
        let new_summary = params["summary"].as_str().map(|s| s.to_string());
        let new_confidence = match params["confidence"].as_str() {
            Some(s) => Some(validate_confidence(s)?),
            None => None,
        };
        let new_status = match params["status"].as_str() {
            Some(s) => Some(validate_status(s)?),
            None => None,
        };
        let new_sensitivity = params["sensitivity"]
            .as_str()
            .map(sensitivity_str_to_sensitivity);

        // Reject empty update
        if new_summary.is_none()
            && new_confidence.is_none()
            && new_status.is_none()
            && new_sensitivity.is_none()
        {
            return Err(MemoryError::InvalidId("no fields to update".into()));
        }

        let mut obs = self
            .obs_repo
            .get_by_id_with_links(id)
            .await?
            .ok_or(MemoryError::ObservationNotFound(id))?;

        // Authorization: caller must have read access to this observation
        self.access_context.check_read_access(
            &obs.scope,
            obs.project_id,
            obs.user_id,
            obs.organization_id,
            &obs.sensitivity,
        )?;

        let old_summary = obs.summary.clone();
        let mut content_changed = false;

        if let Some(summary) = new_summary {
            obs.summary = summary;
            content_changed = true;
        }
        if let Some(conf) = new_confidence {
            obs.confidence = conf;
        }
        if let Some(status) = new_status {
            if !valid_transition(obs.status, status) {
                return Err(MemoryError::InvalidStatusTransition {
                    from: obs.status.to_string(),
                    to: status.to_string(),
                });
            }
            obs.status = status;
        }
        if let Some(sensitivity) = new_sensitivity {
            obs.sensitivity = sensitivity;
        }

        let updated = self.obs_repo.update(&obs).await?;

        // Regenerate embedding if content changed
        if content_changed {
            let provider = create_embedding_provider(
                "mock",
                &ProviderConfig {
                    api_key: None,
                    model: "mock".into(),
                    base_url: None,
                    dimensions: Some(1536),
                },
            )?;
            let embedding_vec = provider.embed(&updated.summary).await?;
            self.embeddings_repo
                .upsert_embedding(updated.id, "mock", 1536, &embedding_vec)
                .await?;
        }

        // Write audit log
        self.audit_repo
            .insert(
                actor,
                None,
                "update",
                Some(id),
                Some(&serde_json::json!({"summary": old_summary})),
                Some(&serde_json::json!({
                    "summary": updated.summary,
                    "confidence": updated.confidence.to_string(),
                    "status": updated.status.to_string(),
                })),
            )
            .await?;

        Ok(serde_json::json!({
            "id": updated.id.to_string(),
            "status": "updated",
        }))
    }

    async fn handle_mark_obsolete(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let actor = params["actor"].as_str().unwrap_or("agent");
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidId("missing id".into()))?;
        let id = Uuid::parse_str(id_str).map_err(|e| MemoryError::InvalidId(e.to_string()))?;
        let reason = params["reason"].as_str().unwrap_or_default();

        let mut obs = self
            .obs_repo
            .get_by_id(id)
            .await?
            .ok_or(MemoryError::ObservationNotFound(id))?;

        // Authorization: caller must have read access
        self.access_context.check_read_access(
            &obs.scope,
            obs.project_id,
            obs.user_id,
            obs.organization_id,
            &obs.sensitivity,
        )?;

        if !valid_transition(obs.status, MemoryStatus::Obsolete) {
            return Err(MemoryError::InvalidStatusTransition {
                from: obs.status.to_string(),
                to: MemoryStatus::Obsolete.to_string(),
            });
        }

        obs.status = MemoryStatus::Obsolete;
        self.obs_repo.update(&obs).await?;

        self.audit_repo
            .insert(
                actor,
                None,
                "mark_obsolete",
                Some(id),
                None,
                Some(&serde_json::json!({"reason": reason})),
            )
            .await?;

        Ok(serde_json::json!({"id": id.to_string(), "status": "obsolete"}))
    }

    async fn handle_consolidate(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let session_id = params["session_id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let project_id = parse_optional_uuid(&params, "project_id");

        // Authorization: consolidation writes observations at project scope
        self.access_context.check_write_access(
            &MemoryScope::Project,
            project_id,
            None,
            None,
            &MemorySensitivity::Internal,
        )?;
        let user_instructions = params["user_instructions"].as_str().map(String::from);
        let limit = params["limit"].as_u64().unwrap_or(200) as i64;

        // Step 1: Load session events
        let events = self
            .events_repo
            .list_by_session(&session_id, limit)
            .await?;

        // Step 2: Load existing observations for conflict detection context
        let existing = self
            .obs_repo
            .list_active_with_entities(project_id)
            .await?;

        // Step 3: Call consolidation provider
        let input = ConsolidationInput {
            session_id: session_id.clone(),
            project_id,
            events: events.clone(),
            existing_observations: existing.clone(),
            user_instructions,
        };

        let candidates = self
            .consolidation_provider
            .consolidate(input)
            .await?;

        // Step 4: Process each candidate observation
        let embedding_provider = create_embedding_provider(
            "mock",
            &ProviderConfig {
                api_key: None,
                model: "mock".into(),
                base_url: None,
                dimensions: Some(1536),
            },
        )?;

        let mut written = Vec::new();
        let mut conflict_count = 0;

        for candidate in &candidates {
            // Build observation from candidate
            let mut obs = Observation::new(
                candidate.scope,
                session_id.clone(),
                candidate.kind,
                candidate.summary.clone(),
                candidate.confidence,
                candidate.sensitivity,
            )?;
            obs.project_id = project_id;
            obs.entities = candidate.entities.clone();
            obs.files = candidate.files.clone();
            obs.commands = candidate.commands.clone();

            // Conflict detection
            let conflicts = detect_conflicts(&obs, &existing);

            if !conflicts.is_empty() {
                obs.status = MemoryStatus::Conflicted;
            } else {
                obs.status = MemoryStatus::Active;
            }

            // Insert observation with links
            let saved = self
                .obs_repo
                .insert_with_links(
                    &obs,
                    &candidate.files,
                    &candidate.entities,
                    &candidate.commands,
                    &[],
                )
                .await?;

            // Insert evidence from source events
            for event_id in &candidate.source_event_ids {
                let mut evidence = memory_core::EvidenceRef::new(
                    obs.id,
                    memory_core::EvidenceSourceType::Message,
                    event_id.to_string(),
                );
                if let Some(ref rationale) = candidate.rationale {
                    evidence = evidence.with_excerpt(rationale.clone());
                }
                self.evidence_repo.insert(&evidence).await?;
            }

            // Generate and store embedding
            let embedding_vec = embedding_provider.embed(&obs.summary).await?;
            self.embeddings_repo
                .upsert_embedding(obs.id, "mock", 1536, &embedding_vec)
                .await?;

            // Write conflict records
            for conflict in &conflicts {
                self.conflicts_repo
                    .insert_conflict(
                        conflict.left_observation_id,
                        conflict.right_observation_id,
                        &conflict.conflict_type.to_string(),
                        &conflict.description,
                    )
                    .await?;
            }
            conflict_count += conflicts.len();

            written.push(serde_json::json!({
                "id": saved.id.to_string(),
                "kind": saved.kind.to_string(),
                "summary": saved.summary,
                "status": saved.status.to_string(),
            }));
        }

        // Step 5: Write audit log
        self.audit_repo
            .insert(
                "consolidation",
                None,
                "consolidate_session",
                None,
                None,
                Some(&serde_json::json!({
                    "session_id": session_id,
                    "events_loaded": events.len(),
                    "candidates_generated": candidates.len(),
                    "observations_written": written.len(),
                    "conflicts_detected": conflict_count,
                })),
            )
            .await?;

        Ok(serde_json::json!({
            "session_id": session_id,
            "status": "completed",
            "events_processed": events.len(),
            "candidates_generated": candidates.len(),
            "observations_written": written.len(),
            "conflicts_detected": conflict_count,
            "observations": written,
        }))
    }

    async fn handle_session_start(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let project_id = parse_optional_uuid(&params, "project_id");

        // Authorization: session start reads project memory
        self.access_context.check_read_access(
            &MemoryScope::Project,
            project_id,
            None,
            None,
            &MemorySensitivity::Internal,
        )?;

        let session_id = params["session_id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let token_budget = params["token_budget"].as_u64().unwrap_or(1000) as usize;

        let provider = create_embedding_provider(
            "mock",
            &ProviderConfig {
                api_key: None,
                model: "mock".into(),
                base_url: None,
                dimensions: Some(1536),
            },
        )?;

        // Run strategic queries to cover different knowledge domains
        let queries = vec![
            ("architecture", "architecture design system structure"),
            ("decision", "key decisions tradeoffs rationale"),
            ("constraint", "constraints requirements limitations"),
            ("preference", "preferences conventions patterns style"),
            ("policy", "policies rules guidelines standards"),
            ("dependency", "dependencies versions libraries packages"),
            ("procedure", "procedures workflows processes setup"),
        ];

        let mut seen_ids = std::collections::HashSet::new();
        let mut collected = Vec::new();
        let chars_per_token = 4;
        let per_query_limit = 5;

        for (_domain, query) in &queries {
            let query_embedding = provider.embed(query).await?;

            let results = self
                .search_repo
                .hybrid_search(SearchParams {
                    query_embedding,
                    text_query: query.to_string(),
                    scope: "project".into(),
                    project_id,
                    kinds: None,
                    files: None,
                    entities: None,
                    limit: per_query_limit,
                    min_confidence: None,
                })
                .await?;

            for scored in results {
                if seen_ids.insert(scored.observation.id) {
                    let item_tokens =
                        scored.observation.summary.len() / chars_per_token + 1;
                    let total_tokens: usize = collected.iter().map(|m: &serde_json::Value| {
                        m["summary"].as_str().unwrap_or("").len() / chars_per_token + 1
                    }).sum();

                    if total_tokens + item_tokens > token_budget {
                        break;
                    }

                    collected.push(serde_json::json!({
                        "id": scored.observation.id.to_string(),
                        "kind": scored.observation.kind.to_string(),
                        "summary": scored.observation.summary,
                        "confidence": scored.observation.confidence.to_string(),
                        "status": scored.observation.status.to_string(),
                        "score": scored.final_score,
                    }));
                }
            }
        }

        let total_tokens: usize = collected.iter().map(|m| {
            m["summary"].as_str().unwrap_or("").len() / chars_per_token + 1
        }).sum();

        Ok(serde_json::json!({
            "session_id": session_id,
            "context": collected,
            "token_estimate": total_tokens,
            "budget_exceeded": total_tokens > token_budget,
            "memory_count": collected.len(),
        }))
    }

    async fn handle_link_file(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let obs_id_str = params["observation_id"]
            .as_str()
            .ok_or(MemoryError::InvalidId("missing observation_id".into()))?;
        let obs_id = Uuid::parse_str(obs_id_str).map_err(|e| MemoryError::InvalidId(e.to_string()))?;
        let file_path = params["file_path"]
            .as_str()
            .ok_or(MemoryError::InvalidScope)?;

        // Verify observation exists and check authorization
        let obs = self
            .obs_repo
            .get_by_id(obs_id)
            .await?
            .ok_or(MemoryError::ObservationNotFound(obs_id))?;
        self.access_context.check_read_access(
            &obs.scope,
            obs.project_id,
            obs.user_id,
            obs.organization_id,
            &obs.sensitivity,
        )?;

        // Insert file link
        self.obs_repo.link_file(obs_id, file_path).await?;

        Ok(serde_json::json!({"status": "linked", "file_path": file_path}))
    }

    async fn handle_list_conflicts(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let project_id = parse_optional_uuid(&params, "project_id");

        // Authorization: must have read access to this project's conflicts
        self.access_context.check_read_access(
            &MemoryScope::Project,
            project_id,
            None,
            None,
            &MemorySensitivity::Internal,
        )?;
        let conflicts = self.conflicts_repo.list_open_conflicts(project_id).await?;

        Ok(serde_json::json!({
            "conflicts": conflicts.iter().map(|c| serde_json::json!({
                "id": c.id.to_string(),
                "left_observation_id": c.left_observation_id.to_string(),
                "right_observation_id": c.right_observation_id.to_string(),
                "conflict_type": c.conflict_type,
                "description": c.description,
                "status": c.status,
            })).collect::<Vec<_>>()
        }))
    }

    async fn handle_resolve_conflict(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let actor = params["actor"].as_str().unwrap_or("agent");
        let project_id = parse_optional_uuid(&params, "project_id");

        // Validate resolution BEFORE database lookup
        let resolution = params["resolution"].as_str().unwrap_or("left_wins");
        match resolution {
            "left_wins" | "right_wins" | "merge" => {}
            _ => {
                return Err(MemoryError::InvalidId(format!(
                    "invalid resolution strategy: {}",
                    resolution
                )))
            }
        }

        // Authorization: resolving conflicts is a write operation
        self.access_context.check_write_access(
            &MemoryScope::Project,
            project_id,
            None,
            None,
            &MemorySensitivity::Internal,
        )?;

        let conflict_id_str = params["conflict_id"]
            .as_str()
            .ok_or(MemoryError::InvalidId("missing conflict_id".into()))?;
        let conflict_id =
            Uuid::parse_str(conflict_id_str).map_err(|e| MemoryError::InvalidId(e.to_string()))?;

        // Find the conflict record
        let conflicts = self.conflicts_repo.list_open_conflicts(None).await?;
        let conflict = conflicts
            .iter()
            .find(|c| c.id == conflict_id)
            .ok_or(MemoryError::InvalidId(format!("conflict not found: {}", conflict_id)))?;

        let left_id = conflict.left_observation_id;
        let right_id = conflict.right_observation_id;

        match resolution {
            "left_wins" => {
                let mut right_obs = self
                    .obs_repo
                    .get_by_id(right_id)
                    .await?
                    .ok_or(MemoryError::ObservationNotFound(right_id))?;
                right_obs.status = MemoryStatus::Superseded;
                right_obs.superseded_by = Some(left_id);
                self.obs_repo.update(&right_obs).await?;
                self.supers_repo
                    .record_supersession(left_id, right_id, Some("left_wins"))
                    .await?;
            }
            "right_wins" => {
                let mut left_obs = self
                    .obs_repo
                    .get_by_id(left_id)
                    .await?
                    .ok_or(MemoryError::ObservationNotFound(left_id))?;
                left_obs.status = MemoryStatus::Superseded;
                left_obs.superseded_by = Some(right_id);
                self.obs_repo.update(&left_obs).await?;
                self.supers_repo
                    .record_supersession(right_id, left_id, Some("right_wins"))
                    .await?;
            }
            "merge" => {
                // Both remain active, just mark conflict resolved
            }
            _ => unreachable!(), // validated above
        }

        self.conflicts_repo
            .resolve_conflict(conflict_id, resolution)
            .await?;

        self.audit_repo
            .insert(
                actor,
                None,
                "resolve_conflict",
                None,
                None,
                Some(&serde_json::json!({
                    "conflict_id": conflict_id.to_string(),
                    "resolution": resolution,
                })),
            )
            .await?;

        Ok(serde_json::json!({"status": "resolved", "resolution": resolution}))
    }

    async fn handle_delete(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let actor = params["actor"].as_str().unwrap_or("agent");
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidId("missing id".into()))?;
        let id = Uuid::parse_str(id_str).map_err(|e| MemoryError::InvalidId(e.to_string()))?;
        let reason = params["reason"].as_str().unwrap_or_default();
        let permanent = params["permanent"].as_bool().unwrap_or(false);

        // Authorization: load observation and check read access before any deletion
        let obs = self
            .obs_repo
            .get_by_id(id)
            .await?
            .ok_or(MemoryError::ObservationNotFound(id))?;
        self.access_context.check_read_access(
            &obs.scope,
            obs.project_id,
            obs.user_id,
            obs.organization_id,
            &obs.sensitivity,
        )?;

        if permanent {
            self.obs_repo.hard_delete(id).await?;
            self.audit_repo
                .insert(
                    actor,
                    None,
                    "hard_delete",
                    Some(id),
                    None,
                    Some(&serde_json::json!({"reason": reason})),
                )
                .await?;
            return Ok(serde_json::json!({"id": id.to_string(), "status": "permanently_deleted"}));
        }

        if !valid_transition(obs.status, MemoryStatus::Deleted) {
            return Err(MemoryError::InvalidStatusTransition {
                from: obs.status.to_string(),
                to: MemoryStatus::Deleted.to_string(),
            });
        }

        self.obs_repo.soft_delete(id).await?;

        self.audit_repo
            .insert(
                actor,
                None,
                "soft_delete",
                Some(id),
                Some(&serde_json::json!({"status": obs.status.to_string()})),
                Some(&serde_json::json!({"reason": reason})),
            )
            .await?;

        Ok(serde_json::json!({"id": id.to_string(), "status": "deleted"}))
    }
}

fn parse_optional_uuid(params: &serde_json::Value, key: &str) -> Option<Uuid> {
    params[key].as_str().and_then(|s| Uuid::parse_str(s).ok())
}

fn validate_confidence(s: &str) -> Result<MemoryConfidence, MemoryError> {
    match s {
        "high" => Ok(MemoryConfidence::High),
        "medium" => Ok(MemoryConfidence::Medium),
        "low" => Ok(MemoryConfidence::Low),
        _ => Err(MemoryError::InvalidId(format!(
            "invalid confidence value: {}",
            s
        ))),
    }
}

fn validate_status(s: &str) -> Result<MemoryStatus, MemoryError> {
    match s {
        "active" => Ok(MemoryStatus::Active),
        "unconfirmed" => Ok(MemoryStatus::Unconfirmed),
        "superseded" => Ok(MemoryStatus::Superseded),
        "obsolete" => Ok(MemoryStatus::Obsolete),
        "conflicted" => Ok(MemoryStatus::Conflicted),
        "deleted" => Ok(MemoryStatus::Deleted),
        _ => Err(MemoryError::InvalidId(format!(
            "invalid status value: {}",
            s
        ))),
    }
}

fn kind_str_to_kind(s: &str) -> MemoryKind {
    use MemoryKind::*;
    match s {
        "decision" => Decision,
        "fact" => Fact,
        "constraint" => Constraint,
        "preference" => Preference,
        "procedure" => Procedure,
        "implementation_detail" => ImplementationDetail,
        "bug" => Bug,
        "fix" => Fix,
        "failed_attempt" => FailedAttempt,
        "todo" => Todo,
        "open_question" => OpenQuestion,
        "dependency" => Dependency,
        "risk" => Risk,
        "policy" => Policy,
        "external_reference" => ExternalReference,
        _ => Fact,
    }
}

fn confidence_str_to_confidence(s: &str) -> MemoryConfidence {
    match s {
        "high" => MemoryConfidence::High,
        "medium" => MemoryConfidence::Medium,
        _ => MemoryConfidence::Low,
    }
}

fn sensitivity_str_to_sensitivity(s: &str) -> MemorySensitivity {
    match s {
        "public" => MemorySensitivity::Public,
        "internal" => MemorySensitivity::Internal,
        "private" => MemorySensitivity::Private,
        "secret" => MemorySensitivity::Secret,
        _ => MemorySensitivity::Internal,
    }
}

fn source_type_str_to_enum(s: &str) -> memory_core::EvidenceSourceType {
    match s {
        "message" => memory_core::EvidenceSourceType::Message,
        "tool_call" => memory_core::EvidenceSourceType::ToolCall,
        "file" => memory_core::EvidenceSourceType::File,
        "terminal" => memory_core::EvidenceSourceType::Terminal,
        "commit" => memory_core::EvidenceSourceType::Commit,
        "issue" => memory_core::EvidenceSourceType::Issue,
        "pull_request" => memory_core::EvidenceSourceType::PullRequest,
        "document" => memory_core::EvidenceSourceType::Document,
        "web" => memory_core::EvidenceSourceType::Web,
        "manual_entry" => memory_core::EvidenceSourceType::ManualEntry,
        _ => memory_core::EvidenceSourceType::ManualEntry,
    }
}
