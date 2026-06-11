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
        let rows = sqlx::query(
            r#"
            WITH vector_matches AS (
                SELECT
                    o.id,
                    1.0 - (e.embedding <=> $1::vector) AS vector_score
                FROM observation_embeddings e
                JOIN observations o ON o.id = e.observation_id
                WHERE
                    o.status = 'active'
                    AND o.scope::text = $2
                    AND ($3::uuid IS NULL OR o.project_id = $3)
                    AND o.sensitivity != 'secret'
                ORDER BY e.embedding <=> $1::vector
                LIMIT 50
            ),
            text_matches AS (
                SELECT
                    o.id,
                    ts_rank_cd(o.search_tsv, plainto_tsquery('english', $4)) AS text_score
                FROM observations o
                WHERE
                    o.status = 'active'
                    AND o.scope::text = $2
                    AND ($3::uuid IS NULL OR o.project_id = $3)
                    AND o.sensitivity != 'secret'
                    AND o.search_tsv @@ plainto_tsquery('english', $4)
                ORDER BY text_score DESC
                LIMIT 50
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
                    COALESCE(tm.text_score, 0.0) AS text_score,
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
                    o.updated_at
                FROM observations o
                LEFT JOIN vector_matches vm ON vm.id = o.id
                LEFT JOIN text_matches tm ON tm.id = o.id
                WHERE vm.id IS NOT NULL OR tm.id IS NOT NULL
            )
            SELECT
                *,
                (
                    COALESCE(vector_score, 0.0) * 0.45 +
                    COALESCE(text_score, 0.0) * 0.30 +
                    confidence_score * 0.15 +
                    recency_score * 0.10
                ) AS final_score
            FROM combined
            ORDER BY final_score DESC
            LIMIT $5
            "#,
        )
        .bind(&params.query_embedding)
        .bind(&params.scope)
        .bind(params.project_id)
        .bind(&params.text_query)
        .bind(params.limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| row_to_scored_observation(r)).collect())
    }
}

fn row_to_scored_observation(row: &sqlx::postgres::PgRow) -> ScoredObservation {
    ScoredObservation {
        observation: Observation {
            id: row.get("id"),
            scope: row.get("scope"),
            project_id: row.get("project_id"),
            user_id: row.get("user_id"),
            organization_id: row.get("organization_id"),
            session_id: row.get("session_id"),
            kind: row.get("kind"),
            summary: row.get("summary"),
            entities: vec![],
            files: vec![],
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
        },
        final_score: row.get::<Option<f64>, _>("final_score").unwrap_or(0.0),
        vector_score: row.get::<Option<f64>, _>("vector_score").unwrap_or(0.0),
        text_score: row.get::<Option<f64>, _>("text_score").unwrap_or(0.0),
    }
}
