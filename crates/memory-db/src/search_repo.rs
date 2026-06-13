use sqlx::{PgPool, Row};
use uuid::Uuid;

use memory_core::{MemoryError, Observation};

#[derive(Debug, Clone)]
pub struct SearchParams {
    pub query_embedding: Vec<f32>,
    pub text_query: String,
    pub scope: String,
    pub project_id: Option<Uuid>,
    pub kinds: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    pub entities: Option<Vec<String>>,
    pub limit: i64,
    pub min_confidence: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScoredObservation {
    pub observation: Observation,
    pub final_score: f64,
    pub vector_score: f64,
    pub text_score: f64,
}

pub struct SearchRepository {
    pool: PgPool,
}

impl SearchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn hybrid_search(
        &self,
        params: SearchParams,
    ) -> Result<Vec<ScoredObservation>, MemoryError> {
        let kinds_ref = params.kinds.as_ref();
        let files_ref = params.files.as_ref();
        let entities_ref = params.entities.as_ref();
        let min_conf = params.min_confidence.as_deref();

        let rows = sqlx::query(
            r#"
            WITH vector_matches AS (
                SELECT
                    o.id,
                    1.0 - (e.embedding <=> $1::vector) AS vector_score
                FROM observation_embeddings e
                JOIN observations o ON o.id = e.observation_id
                WHERE
                    o.status IN ('active', 'unconfirmed', 'conflicted')
                    AND o.scope::text = $2
                    AND ($3::uuid IS NULL OR o.project_id = $3)
                    AND o.sensitivity NOT IN ('secret', 'private')
                    AND ($4::text[] IS NULL OR o.kind::text = ANY($4::text[]))
                ORDER BY e.embedding <=> $1::vector
                LIMIT 100
            ),
            text_matches AS (
                SELECT
                    o.id,
                    ts_rank_cd(o.search_tsv, plainto_tsquery('english', $5)) AS text_score
                FROM observations o
                WHERE
                    o.status IN ('active', 'unconfirmed', 'conflicted')
                    AND o.scope::text = $2
                    AND ($3::uuid IS NULL OR o.project_id = $3)
                    AND o.sensitivity NOT IN ('secret', 'private')
                    AND ($4::text[] IS NULL OR o.kind::text = ANY($4::text[]))
                    AND o.search_tsv @@ plainto_tsquery('english', $5)
                ORDER BY text_score DESC
                LIMIT 100
            ),
            file_match AS (
                SELECT DISTINCT observation_id
                FROM observation_files
                WHERE $6::text[] IS NOT NULL AND file_path = ANY($6::text[])
            ),
            entity_match AS (
                SELECT DISTINCT observation_id
                FROM observation_entities
                WHERE $7::text[] IS NOT NULL AND entity = ANY($7::text[])
            ),
            combined AS (
                SELECT
                    o.id,
                    o.kind,
                    o.summary,
                    o.confidence,
                    o.status,
                    o.created_at,
                    COALESCE(vm.vector_score, 0.0) AS vector_score,
                    COALESCE(tm.text_score, 0.0::float8) AS text_score,
                    CASE o.confidence::text
                        WHEN 'high' THEN 1.0
                        WHEN 'medium' THEN 0.7
                        WHEN 'low' THEN 0.4
                        ELSE 0.5
                    END AS confidence_score,
                    LEAST(
                        1.0,
                        1.0 / (
                            1.0 + EXTRACT(EPOCH FROM (now() - o.created_at)) / 2592000.0
                        )
                    ) AS recency_score,
                    o.scope,
                    o.project_id,
                    o.user_id,
                    o.organization_id,
                    o.session_id,
                    o.sensitivity,
                    o.valid_until,
                    o.last_accessed_at,
                    o.last_confirmed_at,
                    o.superseded_by,
                    o.metadata,
                    o.updated_at,
                    CASE
                        WHEN fm.observation_id IS NOT NULL OR em.observation_id IS NOT NULL THEN 1.0
                        ELSE 0.0
                    END AS file_or_entity_match_score,
                    LEAST(1.0, COALESCE(
                        (SELECT COUNT(*)::float / 5.0 FROM evidence e WHERE e.observation_id = o.id),
                        0.0
                    )) AS evidence_score
                FROM observations o
                LEFT JOIN vector_matches vm ON vm.id = o.id
                LEFT JOIN text_matches tm ON tm.id = o.id
                LEFT JOIN file_match fm ON fm.observation_id = o.id
                LEFT JOIN entity_match em ON em.observation_id = o.id
                WHERE (vm.id IS NOT NULL OR tm.id IS NOT NULL)
                  AND ($4::text[] IS NULL OR o.kind::text = ANY($4::text[]))
                  AND (
                      $8::text IS NULL
                      OR (o.confidence::text = 'high' AND $8::text = 'high')
                      OR (o.confidence::text IN ('high', 'medium') AND $8::text = 'medium')
                      OR ($8::text = 'low')
                  )
            )
            SELECT
                *,
                LEAST(1.0, GREATEST(0.0,
                    COALESCE(vector_score, 0.0) * 0.45
                    + COALESCE(text_score, 0.0) * 0.30
                    + confidence_score * 0.10
                    + recency_score * 0.05
                    + evidence_score * 0.05
                    + file_or_entity_match_score * 0.05
                    - CASE status
                        WHEN 'conflicted' THEN 0.15
                        WHEN 'obsolete' THEN 0.10
                        WHEN 'superseded' THEN 0.05
                        ELSE 0.0
                      END
                )) AS final_score
            FROM combined
            ORDER BY final_score DESC
            LIMIT $9
            "#,
        )
        .bind(&params.query_embedding)
        .bind(&params.scope)
        .bind(params.project_id)
        .bind(kinds_ref)
        .bind(&params.text_query)
        .bind(files_ref)
        .bind(entities_ref)
        .bind(min_conf)
        .bind(params.limit)
        .fetch_all(&self.pool)
        .await
        ?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            results.push(row_to_scored_observation(row)?);
        }
        Ok(results)
    }
}

fn row_to_scored_observation(row: &sqlx::postgres::PgRow) -> Result<ScoredObservation, MemoryError> {
    Ok(ScoredObservation {
        observation: Observation {
            id: row.try_get("id")?,
            scope: row.try_get("scope")?,
            project_id: row.try_get("project_id")?,
            user_id: row.try_get("user_id")?,
            organization_id: row.try_get("organization_id")?,
            session_id: row.try_get("session_id")?,
            kind: row.try_get("kind")?,
            summary: row.try_get("summary")?,
            entities: vec![],
            files: vec![],
            commands: vec![],
            links: vec![],
            confidence: row.try_get("confidence")?,
            sensitivity: row.try_get("sensitivity")?,
            status: row.try_get("status")?,
            evidence: vec![],
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            last_accessed_at: row.try_get("last_accessed_at")?,
            last_confirmed_at: row.try_get("last_confirmed_at")?,
            valid_until: row.try_get("valid_until")?,
            supersedes: vec![],
            superseded_by: row.try_get("superseded_by")?,
            metadata: row.try_get("metadata")?,
        },
        final_score: row.try_get::<Option<f64>, _>("final_score")?.unwrap_or(0.0),
        vector_score: row.try_get::<Option<f64>, _>("vector_score")?.unwrap_or(0.0),
        text_score: row.try_get::<Option<f64>, _>("text_score")?.unwrap_or(0.0),
    })
}
