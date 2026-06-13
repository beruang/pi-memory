use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use memory_core::{
    AccessContext, MemoryConfidence, MemoryKind, MemoryScope, MemorySensitivity,
    MemoryStatus,
};
use memory_db::{
    ConflictsRepository, EvidenceRepository, ObservationsRepository, SearchParams,
    SearchRepository,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub access_context: AccessContext,
}

impl AppState {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            pool,
            access_context: AccessContext::admin(),
        }
    }

    pub fn with_context(pool: sqlx::PgPool, ctx: AccessContext) -> Self {
        Self {
            pool,
            access_context: ctx,
        }
    }
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

pub async fn review_ui() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("review.html"),
    )
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: "0.1.0".into(),
    })
}

#[derive(Deserialize)]
pub struct ListParams {
    pub scope: Option<String>,
    pub project_id: Option<Uuid>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_observations(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let obs_repo = ObservationsRepository::new(state.pool.clone());
    let limit = params.limit.unwrap_or(50).min(500);
    let offset = params.offset.unwrap_or(0);
    let scope = params.scope.as_deref().unwrap_or("project");

    let mem_scope: MemoryScope = match scope.parse() {
        Ok(s) => s,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    // Authorization: must have read access at this scope
    state
        .access_context
        .check_read_access(
            &mem_scope,
            params.project_id,
            None,
            None,
            &MemorySensitivity::Internal,
        )
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let observations = obs_repo
        .list_by_scope(mem_scope, params.project_id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "observations": observations.iter().map(|o| serde_json::json!({
            "id": o.id.to_string(),
            "kind": o.kind.to_string(),
            "summary": o.summary,
            "confidence": o.confidence.to_string(),
            "status": o.status.to_string(),
            "scope": o.scope.to_string(),
            "session_id": o.session_id,
            "created_at": o.created_at.to_string(),
        })).collect::<Vec<_>>(),
        "total": observations.len(),
    })))
}

#[derive(Deserialize)]
pub struct CreateObservationRequest {
    pub scope: String,
    pub session_id: String,
    pub kind: String,
    pub summary: String,
    pub confidence: Option<String>,
    pub sensitivity: Option<String>,
    pub project_id: Option<Uuid>,
    pub evidence: Option<Vec<serde_json::Value>>,
}

pub async fn create_observation(
    State(state): State<AppState>,
    Json(body): Json<CreateObservationRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let obs_repo = ObservationsRepository::new(state.pool.clone());

    let scope: MemoryScope = body.scope.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    // Authorization: must have write access at this scope
    let sensitivity = memory_core::MemorySensitivity::Internal;
    state
        .access_context
        .check_write_access(&scope, body.project_id, None, None, &sensitivity)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let kind = match body.kind.as_str() {
        "fact" => MemoryKind::Fact,
        "decision" => MemoryKind::Decision,
        "constraint" => MemoryKind::Constraint,
        "preference" => MemoryKind::Preference,
        "procedure" => MemoryKind::Procedure,
        "implementation_detail" => MemoryKind::ImplementationDetail,
        "bug" => MemoryKind::Bug,
        "fix" => MemoryKind::Fix,
        "failed_attempt" => MemoryKind::FailedAttempt,
        "todo" => MemoryKind::Todo,
        "open_question" => MemoryKind::OpenQuestion,
        "dependency" => MemoryKind::Dependency,
        "risk" => MemoryKind::Risk,
        "policy" => MemoryKind::Policy,
        "external_reference" => MemoryKind::ExternalReference,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let confidence = match body.confidence.as_deref().unwrap_or("medium") {
        "high" => MemoryConfidence::High,
        "medium" => MemoryConfidence::Medium,
        _ => MemoryConfidence::Low,
    };

    let mut obs = memory_core::Observation::new(
        scope,
        body.session_id,
        kind,
        body.summary,
        confidence,
        memory_core::MemorySensitivity::Internal,
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;
    obs.project_id = body.project_id;

    let saved = obs_repo.insert(&obs).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": saved.id.to_string(),
            "status": "created",
        })),
    ))
}

pub async fn get_observation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let obs_repo = ObservationsRepository::new(state.pool.clone());
    let evidence_repo = EvidenceRepository::new(state.pool.clone());

    let obs = obs_repo
        .get_by_id_with_links(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Authorization: must have read access to this observation
    state
        .access_context
        .check_read_access(
            &obs.scope,
            obs.project_id,
            obs.user_id,
            obs.organization_id,
            &obs.sensitivity,
        )
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let evidence = evidence_repo
        .get_by_observation_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "id": obs.id.to_string(),
        "scope": obs.scope.to_string(),
        "kind": obs.kind.to_string(),
        "summary": obs.summary,
        "confidence": obs.confidence.to_string(),
        "sensitivity": obs.sensitivity.to_string(),
        "status": obs.status.to_string(),
        "session_id": obs.session_id,
        "files": obs.files,
        "entities": obs.entities,
        "evidence_count": evidence.len(),
        "evidence": evidence.iter().map(|e| serde_json::json!({
            "id": e.id.to_string(),
            "source_type": e.source_type.to_string(),
            "source_id": e.source_id,
            "excerpt": e.excerpt,
        })).collect::<Vec<_>>(),
        "created_at": obs.created_at.to_string(),
        "updated_at": obs.updated_at.to_string(),
    })))
}

pub async fn update_observation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let obs_repo = ObservationsRepository::new(state.pool.clone());

    let mut obs = obs_repo
        .get_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Authorization: must have read access to this observation
    state
        .access_context
        .check_read_access(
            &obs.scope,
            obs.project_id,
            obs.user_id,
            obs.organization_id,
            &obs.sensitivity,
        )
        .map_err(|_| StatusCode::FORBIDDEN)?;

    if let Some(summary) = body["summary"].as_str() {
        obs.summary = summary.to_string();
    }
    if let Some(conf_str) = body["confidence"].as_str() {
        obs.confidence = match conf_str {
            "high" => MemoryConfidence::High,
            "medium" => MemoryConfidence::Medium,
            _ => MemoryConfidence::Low,
        };
    }
    if let Some(status_str) = body["status"].as_str() {
        obs.status = match status_str {
            "active" => MemoryStatus::Active,
            "unconfirmed" => MemoryStatus::Unconfirmed,
            "obsolete" => MemoryStatus::Obsolete,
            "deleted" => MemoryStatus::Deleted,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
    }

    obs_repo
        .update(&obs)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "id": id.to_string(),
        "status": "updated",
    })))
}

#[derive(Deserialize)]
pub struct DeleteParams {
    pub permanent: Option<bool>,
}

pub async fn delete_observation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<DeleteParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let obs_repo = ObservationsRepository::new(state.pool.clone());

    // Authorization: must have read access to this observation
    let obs = obs_repo
        .get_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    state
        .access_context
        .check_read_access(
            &obs.scope,
            obs.project_id,
            obs.user_id,
            obs.organization_id,
            &obs.sensitivity,
        )
        .map_err(|_| StatusCode::FORBIDDEN)?;

    if params.permanent.unwrap_or(false) {
        obs_repo
            .hard_delete(id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(serde_json::json!({
            "id": id.to_string(),
            "status": "permanently_deleted",
        })))
    } else {
        obs_repo
            .soft_delete(id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(serde_json::json!({
            "id": id.to_string(),
            "status": "deleted",
        })))
    }
}

#[derive(Deserialize)]
pub struct SearchQueryParams {
    pub q: String,
    pub scope: Option<String>,
    pub project_id: Option<Uuid>,
    pub limit: Option<i64>,
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let search_repo = SearchRepository::new(state.pool.clone());

    // Authorization: must have read access at this scope
    let scope = params.scope.as_deref().unwrap_or("project");
    let mem_scope: MemoryScope = scope.parse().unwrap_or(MemoryScope::Project);
    state
        .access_context
        .check_read_access(
            &mem_scope,
            params.project_id,
            None,
            None,
            &MemorySensitivity::Internal,
        )
        .map_err(|_| StatusCode::FORBIDDEN)?;

    // Use mock embedding provider for API search
    let provider = memory_providers::create_embedding_provider(
        "mock",
        &memory_providers::ProviderConfig {
            api_key: None,
            model: "mock".into(),
            base_url: None,
            dimensions: Some(1536),
        },
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let query_embedding = provider
        .embed(&params.q)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let results = search_repo
        .hybrid_search(SearchParams {
            query_embedding,
            text_query: params.q.clone(),
            scope: params.scope.unwrap_or_else(|| "project".into()),
            project_id: params.project_id,
            kinds: None,
            files: None,
            entities: None,
            limit: params.limit.unwrap_or(10).min(100),
            min_confidence: None,
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "query": params.q,
        "results": results.iter().map(|r| serde_json::json!({
            "id": r.observation.id.to_string(),
            "kind": r.observation.kind.to_string(),
            "summary": r.observation.summary,
            "confidence": r.observation.confidence.to_string(),
            "score": r.final_score,
        })).collect::<Vec<_>>(),
    })))
}

pub async fn list_conflicts(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conflicts_repo = ConflictsRepository::new(state.pool.clone());

    // Authorization: must have read access to this project's conflicts
    state
        .access_context
        .check_read_access(
            &MemoryScope::Project,
            params.project_id,
            None,
            None,
            &MemorySensitivity::Internal,
        )
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let conflicts = conflicts_repo
        .list_open_conflicts(params.project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "conflicts": conflicts.iter().map(|c| serde_json::json!({
            "id": c.id.to_string(),
            "left_observation_id": c.left_observation_id.to_string(),
            "right_observation_id": c.right_observation_id.to_string(),
            "conflict_type": c.conflict_type,
            "description": c.description,
            "status": c.status,
            "created_at": c.created_at.to_string(),
        })).collect::<Vec<_>>(),
    })))
}

pub async fn resolve_conflict(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conflicts_repo = ConflictsRepository::new(state.pool.clone());
    let resolution = body["resolution"].as_str().unwrap_or("left_wins");

    // Authorization: resolving conflicts is a write operation
    state
        .access_context
        .check_write_access(
            &MemoryScope::Project,
            None,
            None,
            None,
            &MemorySensitivity::Internal,
        )
        .map_err(|_| StatusCode::FORBIDDEN)?;

    conflicts_repo
        .resolve_conflict(id, resolution)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "conflict_id": id.to_string(),
        "status": "resolved",
        "resolution": resolution,
    })))
}
