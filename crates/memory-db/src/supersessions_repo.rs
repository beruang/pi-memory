use sqlx::{PgPool, Row};
use uuid::Uuid;

use memory_core::MemoryError;

pub struct SupersessionsRepository {
    pool: PgPool,
}

impl SupersessionsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Record that `newer_id` supersedes `older_id`.
    pub async fn record_supersession(
        &self,
        newer_id: Uuid,
        older_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), MemoryError> {
        sqlx::query(
            r#"
            INSERT INTO observation_supersessions (newer_observation_id, older_observation_id, reason)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(newer_id)
        .bind(older_id)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(())
    }

    /// List older observation IDs superseded by `observation_id`.
    pub async fn list_for_observation(
        &self,
        observation_id: Uuid,
    ) -> Result<Vec<Uuid>, MemoryError> {
        let rows = sqlx::query(
            r#"
            SELECT older_observation_id FROM observation_supersessions
            WHERE newer_observation_id = $1
            "#,
        )
        .bind(observation_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| r.get("older_observation_id")).collect())
    }

    /// Delete all supersession links for `observation_id` (used before hard-delete).
    pub async fn delete_for_observation(&self, observation_id: Uuid) -> Result<(), MemoryError> {
        sqlx::query(
            r#"
            DELETE FROM observation_supersessions
            WHERE newer_observation_id = $1 OR older_observation_id = $1
            "#,
        )
        .bind(observation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_supers_repo_new() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost").unwrap();
        let _repo = SupersessionsRepository::new(pool);
    }
}
