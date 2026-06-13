use sqlx::{PgPool, Row};
use uuid::Uuid;

use memory_core::MemoryError;

#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub id: Uuid,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub action: String,
    pub observation_id: Option<Uuid>,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub created_at: time::OffsetDateTime,
}

pub struct AuditRepository {
    pool: PgPool,
}

impl AuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        actor_type: &str,
        actor_id: Option<&str>,
        action: &str,
        observation_id: Option<Uuid>,
        before: Option<&serde_json::Value>,
        after: Option<&serde_json::Value>,
    ) -> Result<AuditRecord, MemoryError> {
        let row = sqlx::query(
            r#"
            INSERT INTO memory_audit_log (actor_type, actor_id, action, observation_id, before, after)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(actor_type)
        .bind(actor_id)
        .bind(action)
        .bind(observation_id)
        .bind(before.cloned())
        .bind(after.cloned())
        .fetch_one(&self.pool)
        .await
        ?;

        Ok(row_to_audit(&row))
    }

    pub async fn list_for_observation(
        &self,
        observation_id: Uuid,
    ) -> Result<Vec<AuditRecord>, MemoryError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM memory_audit_log
            WHERE observation_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(observation_id)
        .fetch_all(&self.pool)
        .await
        ?;

        Ok(rows.iter().map(row_to_audit).collect())
    }

    pub async fn list_recent(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditRecord>, MemoryError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM memory_audit_log
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        ?;

        Ok(rows.iter().map(row_to_audit).collect())
    }
}

fn row_to_audit(row: &sqlx::postgres::PgRow) -> AuditRecord {
    let before: Option<serde_json::Value> = row.get("before");
    let after: Option<serde_json::Value> = row.get("after");
    AuditRecord {
        id: row.get("id"),
        actor_type: row.get("actor_type"),
        actor_id: row.get("actor_id"),
        action: row.get("action"),
        observation_id: row.get("observation_id"),
        before,
        after,
        created_at: row.get("created_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_repo_new() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost").unwrap();
        let _repo = AuditRepository::new(pool);
    }
}
