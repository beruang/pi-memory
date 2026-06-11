use sqlx::{PgPool, Row};
use uuid::Uuid;

use memory_core::{MemoryError, MemoryScope, Observation};

pub struct ObservationsRepository {
    pool: PgPool,
}

impl ObservationsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, obs: &Observation) -> Result<Observation, MemoryError> {
        let row = sqlx::query(
            r#"
            INSERT INTO observations (
                id, scope, project_id, user_id, organization_id, session_id,
                kind, summary, confidence, sensitivity, status,
                valid_until, last_accessed_at, last_confirmed_at,
                superseded_by, metadata, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#,
        )
        .bind(obs.id)
        .bind(obs.scope)
        .bind(obs.project_id)
        .bind(obs.user_id)
        .bind(obs.organization_id)
        .bind(&obs.session_id)
        .bind(obs.kind)
        .bind(&obs.summary)
        .bind(obs.confidence)
        .bind(obs.sensitivity)
        .bind(obs.status)
        .bind(obs.valid_until)
        .bind(obs.last_accessed_at)
        .bind(obs.last_confirmed_at)
        .bind(obs.superseded_by)
        .bind(&obs.metadata)
        .bind(obs.created_at)
        .bind(obs.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(row_to_observation(&row))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Observation>, MemoryError> {
        let row = sqlx::query(r#"SELECT * FROM observations WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(row.as_ref().map(|r| row_to_observation(r)))
    }

    pub async fn update(&self, obs: &Observation) -> Result<Observation, MemoryError> {
        let row = sqlx::query(
            r#"
            UPDATE observations SET
                summary = $2, confidence = $3, sensitivity = $4, status = $5,
                valid_until = $6, last_accessed_at = $7, last_confirmed_at = $8,
                superseded_by = $9, metadata = $10, updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(obs.id)
        .bind(&obs.summary)
        .bind(obs.confidence)
        .bind(obs.sensitivity)
        .bind(obs.status)
        .bind(obs.valid_until)
        .bind(obs.last_accessed_at)
        .bind(obs.last_confirmed_at)
        .bind(obs.superseded_by)
        .bind(&obs.metadata)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?
        .ok_or(MemoryError::ObservationNotFound(obs.id))?;

        Ok(row_to_observation(&row))
    }

    pub async fn list_by_scope(
        &self,
        scope: MemoryScope,
        project_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Observation>, MemoryError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM observations
            WHERE scope = $1
              AND ($2::uuid IS NULL OR project_id = $2)
              AND status != 'deleted'
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(scope)
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| row_to_observation(r)).collect())
    }

    pub async fn soft_delete(&self, id: Uuid) -> Result<(), MemoryError> {
        let result = sqlx::query(
            r#"UPDATE observations SET status = 'deleted', updated_at = now() WHERE id = $1"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(MemoryError::ObservationNotFound(id));
        }
        Ok(())
    }
}

fn row_to_observation(row: &sqlx::postgres::PgRow) -> Observation {
    Observation {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_new() {
        // Just verifying construction works without a pool
        let _repo = ObservationsRepository::new(PgPool::connect_lazy("postgres://localhost").unwrap());
    }
}
