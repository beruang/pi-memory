use sqlx::{PgPool, Row};
use time::OffsetDateTime;

use memory_core::{MemoryError, SessionEvent};

pub struct EventsRepository {
    pool: PgPool,
}

impl EventsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, event: &SessionEvent) -> Result<SessionEvent, MemoryError> {
        sqlx::query(
            r#"
            INSERT INTO session_events (id, session_id, project_id, user_id, organization_id, event_type, payload, sensitivity, redacted, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(event.id)
        .bind(&event.session_id)
        .bind(event.project_id)
        .bind(event.user_id)
        .bind(event.organization_id)
        .bind(&event.event_type)
        .bind(&event.payload)
        .bind(event.sensitivity)
        .bind(event.redacted)
        .bind(event.created_at)
        .execute(&self.pool)
        .await?;

        Ok(event.clone())
    }

    pub async fn list_by_session(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<SessionEvent>, MemoryError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM session_events
            WHERE session_id = $1
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_event).collect())
    }

    pub async fn list_by_session_since(
        &self,
        session_id: &str,
        since: OffsetDateTime,
        limit: i64,
    ) -> Result<Vec<SessionEvent>, MemoryError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM session_events
            WHERE session_id = $1 AND created_at >= $2
            ORDER BY created_at ASC
            LIMIT $3
            "#,
        )
        .bind(session_id)
        .bind(since)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_event).collect())
    }

    pub async fn delete_older_than(
        &self,
        cutoff: OffsetDateTime,
    ) -> Result<u64, MemoryError> {
        let result = sqlx::query(
            r#"DELETE FROM session_events WHERE created_at < $1"#,
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete_by_session(&self, session_id: &str) -> Result<u64, MemoryError> {
        let result = sqlx::query(
            r#"DELETE FROM session_events WHERE session_id = $1"#,
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn count_by_session(&self, session_id: &str) -> Result<i64, MemoryError> {
        let row = sqlx::query(
            r#"SELECT COUNT(*) as cnt FROM session_events WHERE session_id = $1"#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get::<i64, _>("cnt"))
    }
}

fn row_to_event(row: &sqlx::postgres::PgRow) -> SessionEvent {
    SessionEvent {
        id: row.get("id"),
        session_id: row.get("session_id"),
        project_id: row.get("project_id"),
        user_id: row.get("user_id"),
        organization_id: row.get("organization_id"),
        event_type: row.get("event_type"),
        payload: row.get("payload"),
        sensitivity: row.get("sensitivity"),
        redacted: row.get("redacted"),
        created_at: row.get("created_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_events_repo_new() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost").unwrap();
        let _repo = EventsRepository::new(pool);
    }
}
