use memory_core::{
    detect_conflicts, scan_for_secrets, strip_private_blocks, valid_transition, ConflictRecord,
    MemoryConfidence, MemoryError, MemoryKind, MemoryScope, MemorySensitivity, MemoryStatus,
    Observation, PrivateBlockRange,
};
use memory_db::{
    AuditRepository, ConflictsRepository, EmbeddingsRepository, EvidenceRepository,
    ObservationsRepository, SearchParams, SearchRepository, SupersessionsRepository,
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
    ) -> Self {
        Self {
            obs_repo,
            evidence_repo,
            embeddings_repo,
            search_repo,
            conflicts_repo,
            audit_repo,
            supers_repo,
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
            _ => Err(MemoryError::InvalidScope),
        }
    }

    async fn handle_recall(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let task = params["task"].as_str().unwrap_or_default().to_string();
        let scope = params["scope"].as_str().unwrap_or("project").to_string();
        let project_id = parse_optional_uuid(&params, "project_id");
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
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let id = Uuid::parse_str(id_str).map_err(|_| MemoryError::InvalidScope)?;

        let obs = self
            .obs_repo
            .get_by_id_with_links(id)
            .await?
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

        // Step 1: Strip private blocks
        let (clean_summary, redacted_ranges): (String, Vec<PrivateBlockRange>) =
            strip_private_blocks(&raw_summary);
        let had_private_blocks = !redacted_ranges.is_empty();

        // Step 2: Reject if secrets detected
        if !scan_for_secrets(&clean_summary).is_empty() {
            return Err(MemoryError::SecretContentRejected);
        }

        // Step 3: Classify sensitivity (upgrade to Private if private blocks were stripped)
        let mut sensitivity = sensitivity_str_to_sensitivity(sensitivity_str);
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
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let id = Uuid::parse_str(id_str).map_err(|_| MemoryError::InvalidScope)?;

        let mut obs = self
            .obs_repo
            .get_by_id_with_links(id)
            .await?
            .ok_or(MemoryError::ObservationNotFound(id))?;

        let old_summary = obs.summary.clone();
        let mut content_changed = false;

        if let Some(summary) = params["summary"].as_str() {
            obs.summary = summary.to_string();
            content_changed = true;
        }
        if let Some(conf_str) = params["confidence"].as_str() {
            obs.confidence = confidence_str_to_confidence(conf_str);
        }
        if let Some(status_str) = params["status"].as_str() {
            let new_status = status_str_to_status(status_str);
            if !valid_transition(obs.status, new_status) {
                return Err(MemoryError::InvalidStatusTransition {
                    from: obs.status.to_string(),
                    to: new_status.to_string(),
                });
            }
            obs.status = new_status;
        }
        if let Some(sensitivity_str) = params["sensitivity"].as_str() {
            obs.sensitivity = sensitivity_str_to_sensitivity(sensitivity_str);
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
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let id = Uuid::parse_str(id_str).map_err(|_| MemoryError::InvalidScope)?;
        let reason = params["reason"].as_str().unwrap_or_default();

        let mut obs = self
            .obs_repo
            .get_by_id(id)
            .await?
            .ok_or(MemoryError::ObservationNotFound(id))?;

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
        let _session_id = params["session_id"].as_str().unwrap_or_default();
        Ok(serde_json::json!({
            "status": "queued",
            "message": "Consolidation queued for background processing.",
        }))
    }

    async fn handle_link_file(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let obs_id_str = params["observation_id"]
            .as_str()
            .ok_or(MemoryError::InvalidScope)?;
        let obs_id = Uuid::parse_str(obs_id_str).map_err(|_| MemoryError::InvalidScope)?;
        let file_path = params["file_path"]
            .as_str()
            .ok_or(MemoryError::InvalidScope)?;

        // Verify observation exists
        let _ = self
            .obs_repo
            .get_by_id(obs_id)
            .await?
            .ok_or(MemoryError::ObservationNotFound(obs_id))?;

        // Insert file link
        self.obs_repo.link_file(obs_id, file_path).await?;

        Ok(serde_json::json!({"status": "linked", "file_path": file_path}))
    }

    async fn handle_list_conflicts(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
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

    async fn handle_resolve_conflict(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MemoryError> {
        let actor = params["actor"].as_str().unwrap_or("agent");
        let conflict_id_str = params["conflict_id"]
            .as_str()
            .ok_or(MemoryError::InvalidScope)?;
        let conflict_id =
            Uuid::parse_str(conflict_id_str).map_err(|_| MemoryError::InvalidScope)?;
        let resolution = params["resolution"].as_str().unwrap_or("left_wins");

        // Find the conflict record
        let conflicts = self.conflicts_repo.list_open_conflicts(None).await?;
        let conflict = conflicts
            .iter()
            .find(|c| c.id == conflict_id)
            .ok_or(MemoryError::InvalidScope)?;

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
            _ => {}
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
        let id_str = params["id"].as_str().ok_or(MemoryError::InvalidScope)?;
        let id = Uuid::parse_str(id_str).map_err(|_| MemoryError::InvalidScope)?;
        let reason = params["reason"].as_str().unwrap_or_default();

        let obs = self
            .obs_repo
            .get_by_id(id)
            .await?
            .ok_or(MemoryError::ObservationNotFound(id))?;

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
                "delete",
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

fn status_str_to_status(s: &str) -> MemoryStatus {
    match s {
        "active" => MemoryStatus::Active,
        "unconfirmed" => MemoryStatus::Unconfirmed,
        "superseded" => MemoryStatus::Superseded,
        "obsolete" => MemoryStatus::Obsolete,
        "conflicted" => MemoryStatus::Conflicted,
        "deleted" => MemoryStatus::Deleted,
        _ => MemoryStatus::Active,
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
