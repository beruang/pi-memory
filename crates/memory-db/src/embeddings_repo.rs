use sqlx::{PgPool, Row};
use uuid::Uuid;

use memory_core::MemoryError;

pub struct EmbeddingsRepository {
    pool: PgPool,
}

impl EmbeddingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_embedding(
        &self,
        observation_id: Uuid,
        model: &str,
        dimensions: i32,
        embedding: &[f32],
    ) -> Result<(), MemoryError> {
        sqlx::query(
            r#"
            INSERT INTO observation_embeddings (observation_id, model, dimensions, embedding)
            VALUES ($1, $2, $3, $4::vector)
            ON CONFLICT (observation_id, model)
            DO UPDATE SET embedding = EXCLUDED.embedding, dimensions = EXCLUDED.dimensions, created_at = now()
            "#,
        )
        .bind(observation_id)
        .bind(model)
        .bind(dimensions)
        .bind(embedding)
        .execute(&self.pool)
        .await
        ?;

        Ok(())
    }

    pub async fn vector_search(
        &self,
        query_embedding: &[f32],
        model: &str,
        scope: &str,
        project_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<(Uuid, f64)>, MemoryError> {
        let rows = sqlx::query(
            r#"
            SELECT
                e.observation_id,
                1.0 - (e.embedding <=> $1::vector) AS vector_score
            FROM observation_embeddings e
            JOIN observations o ON o.id = e.observation_id
            WHERE
                o.status = 'active'
                AND o.scope::text = $2
                AND ($3::uuid IS NULL OR o.project_id = $3)
                AND o.sensitivity != 'secret'
                AND e.model = $4
            ORDER BY e.embedding <=> $1::vector
            LIMIT $5
            "#,
        )
        .bind(query_embedding)
        .bind(scope)
        .bind(project_id)
        .bind(model)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        ?;

        Ok(rows
            .iter()
            .map(|r| {
                let obs_id: Uuid = r.get("observation_id");
                let score: Option<f64> = r.get("vector_score");
                (obs_id, score.unwrap_or(0.0))
            })
            .collect())
    }

    pub async fn delete_by_observation_id(&self, observation_id: Uuid) -> Result<(), MemoryError> {
        sqlx::query(r#"DELETE FROM observation_embeddings WHERE observation_id = $1"#)
            .bind(observation_id)
            .execute(&self.pool)
            .await
            ?;

        Ok(())
    }
}
