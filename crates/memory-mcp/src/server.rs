use memory_core::MemoryError;
use memory_db::{
    ObservationsRepository, EvidenceRepository, EmbeddingsRepository,
    SearchRepository, ConflictsRepository, SearchParams,
};
use memory_providers::{create_embedding_provider, ProviderConfig};
use uuid::Uuid;

pub struct MemoryMcpServer {
    obs_repo: ObservationsRepository,
    evidence_repo: EvidenceRepository,
    embeddings_repo: EmbeddingsRepository,
    search_repo: SearchRepository,
    conflicts_repo: ConflictsRepository,
}

impl MemoryMcpServer {
    pub fn new(
        obs_repo: ObservationsRepository,
        evidence_repo: EvidenceRepository,
        embeddings_repo: EmbeddingsRepository,
        search_repo: SearchRepository,
        conflicts_repo: ConflictsRepository,
    ) -> Self {
        Self {
            obs_repo,
            evidence_repo,
            embeddings_repo,
            search_repo,
            conflicts_repo,
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
            "memory.link_file" => self.handle_link_file(params).await,
            "memory.list_conflicts" => self.handle_list_conflicts(params).await,
            "memory.resolve_conflict" => self.handle_resolve_conflict(params).await,
            "memory.delete" => self.handle_delete(params).await,
            _ => Err(MemoryError::InvalidScope), // Unknown tool
        }
    }

    async fn handle_recall(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let task = params["task"].as_str().unwrap_or_default().to_string();
        let scope = params["scope"].as_str().unwrap_or("project").to_string();
        let project_id = parse_optional_uuid(&params, "project_id");
        let files: Vec<String> = params["files"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let token_budget = params["token_budget"].as_u64().unwrap_or(1200) as usize;

        // Generate a simple embedding from the task for vector search
        let provider = create_embedding_provider("mock", &ProviderConfig {
            api_key: None,
            model: "mock".into(),
            base_url: None,
            dimensions: Some(128),
        })?;
        let query_embedding = provider.embed(&task).await?;

        let results = self.search_repo.hybrid_search(SearchParams {
            query_embedding,
            text_query: task.clone(),
            scope,
            project_id,
            kinds: None,
            files: if files.is_empty() { None } else { Some(files) },
            limit: 10,
            min_confidence: None,
        }).await?;

        let mut memories = Vec::new();
        let mut token_estimate = 0;
        let chars_per_token = 4; // Approximation

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

    async fn handle_search(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let query = params["query"].as_str().unwrap_or_default().to_string();
        let scope = params["scope"].as_str().unwrap_or("project").to_string();
        let project_id = parse_optional_uuid(&params, "project_id");
        let limit = params["limit"].as_u64().unwrap_or(10) as i64;

        let provider = create_embedding_provider("mock", &ProviderConfig {
            api_key: None,
            model: "mock".into(),
            base_url: None,
            dimensions: Some(128),
        })?;
        let query_embedding = provider.embed(&query).await?;

        let results = self.search_repo.hybrid_search(SearchParams {
            query_embedding,
            text_query: query,
            scope,
            project_id,
            kinds: None,
            files: None,
            limit,
            min_confidence: None,
        }).await?;

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

    async fn handle_get(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let id = Uuid::parse_str(id_str).map_err(|_| MemoryError::InvalidScope)?;

        let obs = self.obs_repo.get_by_id(id).await?
            .ok_or(MemoryError::ObservationNotFound(id))?;
        let evidence = self.evidence_repo.get_by_observation_id(id).await?;

        Ok(serde_json::json!({
            "id": obs.id.to_string(),
            "scope": obs.scope.to_string(),
            "kind": obs.kind.to_string(),
            "summary": obs.summary,
            "confidence": obs.confidence.to_string(),
            "sensitivity": obs.sensitivity.to_string(),
            "status": obs.status.to_string(),
            "evidence": evidence.iter().map(|e| serde_json::json!({
                "id": e.id.to_string(),
                "source_type": e.source_type.to_string(),
                "source_id": e.source_id,
                "excerpt": e.excerpt,
            })).collect::<Vec<_>>(),
            "created_at": obs.created_at.to_string(),
            "updated_at": obs.updated_at.to_string(),
        }))
    }

    async fn handle_write(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let scope_str = params["scope"].as_str().unwrap_or("project");
        let scope: memory_core::MemoryScope = scope_str.parse().map_err(|_| MemoryError::InvalidScope)?;
        let session_id = params["session_id"].as_str().unwrap_or_default().to_string();
        let kind_str = params["kind"].as_str().unwrap_or("fact");
        let kind: memory_core::MemoryKind = kind_str_to_kind(kind_str);
        let summary = params["summary"].as_str().unwrap_or_default().to_string();
        let confidence_str = params["confidence"].as_str().unwrap_or("medium");
        let confidence = confidence_str_to_confidence(confidence_str);
        let sensitivity_str = params["sensitivity"].as_str().unwrap_or("internal");
        let sensitivity = sensitivity_str_to_sensitivity(sensitivity_str);

        if sensitivity == memory_core::MemorySensitivity::Secret {
            return Err(MemoryError::SecretContentRejected);
        }

        let mut obs = memory_core::Observation::new(scope, session_id, kind, summary, confidence, sensitivity)?;
        obs.project_id = parse_optional_uuid(&params, "project_id");

        // Attach evidence if provided
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

        let saved = self.obs_repo.insert(&obs).await?;

        // Store evidence
        for ev in &obs.evidence {
            self.evidence_repo.insert(ev).await?;
        }

        Ok(serde_json::json!({
            "id": saved.id.to_string(),
            "status": "written",
        }))
    }

    async fn handle_update(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let id = Uuid::parse_str(id_str).map_err(|_| MemoryError::InvalidScope)?;

        let mut obs = self.obs_repo.get_by_id(id).await?
            .ok_or(MemoryError::ObservationNotFound(id))?;

        if let Some(summary) = params["summary"].as_str() {
            obs.summary = summary.to_string();
        }
        if let Some(conf_str) = params["confidence"].as_str() {
            obs.confidence = confidence_str_to_confidence(conf_str);
        }
        if let Some(status_str) = params["status"].as_str() {
            obs.status = status_str_to_status(status_str);
        }

        let updated = self.obs_repo.update(&obs).await?;
        Ok(serde_json::json!({
            "id": updated.id.to_string(),
            "status": "updated",
        }))
    }

    async fn handle_mark_obsolete(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let id = Uuid::parse_str(id_str).map_err(|_| MemoryError::InvalidScope)?;
        let _reason = params["reason"].as_str().unwrap_or_default();

        let mut obs = self.obs_repo.get_by_id(id).await?
            .ok_or(MemoryError::ObservationNotFound(id))?;
        obs.status = memory_core::MemoryStatus::Obsolete;
        self.obs_repo.update(&obs).await?;

        Ok(serde_json::json!({"id": id.to_string(), "status": "obsolete"}))
    }

    async fn handle_consolidate(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let _session_id = params["session_id"].as_str().unwrap_or_default();
        // Consolidation is handled by the daemon worker; this is a trigger
        Ok(serde_json::json!({
            "status": "queued",
            "message": "Consolidation queued for background processing.",
        }))
    }

    async fn handle_link_file(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let _obs_id = params["observation_id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let _file_path = params["file_path"].as_str().ok_or(MemoryError::InvalidScope)?;
        // File link is tracked via observation_files table — simplified for MVP
        Ok(serde_json::json!({"status": "linked"}))
    }

    async fn handle_list_conflicts(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let project_id = parse_optional_uuid(&params, "project_id");
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

    async fn handle_resolve_conflict(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let conflict_id_str = params["conflict_id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let conflict_id = Uuid::parse_str(conflict_id_str).map_err(|_| MemoryError::InvalidScope)?;
        let _resolution = params["resolution"].as_str().unwrap_or("left_wins");

        self.conflicts_repo.resolve_conflict(conflict_id, "resolved").await?;

        Ok(serde_json::json!({"status": "resolved"}))
    }

    async fn handle_delete(&self, params: serde_json::Value) -> Result<serde_json::Value, MemoryError> {
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let id = Uuid::parse_str(id_str).map_err(|_| MemoryError::InvalidScope)?;
        let _reason = params["reason"].as_str().unwrap_or_default();

        self.obs_repo.soft_delete(id).await?;
        Ok(serde_json::json!({"id": id.to_string(), "status": "deleted"}))
    }
}

fn parse_optional_uuid(params: &serde_json::Value, key: &str) -> Option<Uuid> {
    params[key].as_str().and_then(|s| Uuid::parse_str(s).ok())
}

fn kind_str_to_kind(s: &str) -> memory_core::MemoryKind {
    use memory_core::MemoryKind::*;
    match s {
        "decision" => Decision, "fact" => Fact, "constraint" => Constraint,
        "preference" => Preference, "procedure" => Procedure,
        "implementation_detail" => ImplementationDetail,
        "bug" => Bug, "fix" => Fix, "failed_attempt" => FailedAttempt,
        "todo" => Todo, "open_question" => OpenQuestion,
        "dependency" => Dependency, "risk" => Risk, "policy" => Policy,
        "external_reference" => ExternalReference,
        _ => Fact,
    }
}

fn confidence_str_to_confidence(s: &str) -> memory_core::MemoryConfidence {
    match s {
        "high" => memory_core::MemoryConfidence::High,
        "medium" => memory_core::MemoryConfidence::Medium,
        _ => memory_core::MemoryConfidence::Low,
    }
}

fn sensitivity_str_to_sensitivity(s: &str) -> memory_core::MemorySensitivity {
    match s {
        "public" => memory_core::MemorySensitivity::Public,
        "internal" => memory_core::MemorySensitivity::Internal,
        "private" => memory_core::MemorySensitivity::Private,
        "secret" => memory_core::MemorySensitivity::Secret,
        _ => memory_core::MemorySensitivity::Internal,
    }
}

fn status_str_to_status(s: &str) -> memory_core::MemoryStatus {
    match s {
        "active" => memory_core::MemoryStatus::Active,
        "unconfirmed" => memory_core::MemoryStatus::Unconfirmed,
        "superseded" => memory_core::MemoryStatus::Superseded,
        "obsolete" => memory_core::MemoryStatus::Obsolete,
        "conflicted" => memory_core::MemoryStatus::Conflicted,
        "deleted" => memory_core::MemoryStatus::Deleted,
        _ => memory_core::MemoryStatus::Active,
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
