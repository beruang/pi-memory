use sqlx::{PgPool, Row};
use uuid::Uuid;

use memory_core::MemoryError;

#[derive(Debug, Clone)]
pub struct ConflictRecord {
    pub id: Uuid,
    pub left_observation_id: Uuid,
    pub right_observation_id: Uuid,
    pub conflict_type: String,
    pub description: String,
    pub status: String,
    pub created_at: time::OffsetDateTime,
    pub resolved_at: Option<time::OffsetDateTime>,
}

pub struct ConflictsRepository {
    pool: PgPool,
}

impl ConflictsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_conflict(
        &self,
        left_observation_id: Uuid,
        right_observation_id: Uuid,
        conflict_type: &str,
        description: &str,
    ) -> Result<ConflictRecord, MemoryError> {
        let row = sqlx::query(
            r#"
            INSERT INTO observation_conflicts (left_observation_id, right_observation_id, conflict_type, description)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(left_observation_id)
        .bind(right_observation_id)
        .bind(conflict_type)
        .bind(description)
        .fetch_one(&self.pool)
        .await
        ?;

        Ok(row_to_conflict(&row))
    }

    pub async fn list_open_conflicts(
        &self,
        project_id: Option<Uuid>,
    ) -> Result<Vec<ConflictRecord>, MemoryError> {
        let rows = if let Some(pid) = project_id {
            sqlx::query(
                r#"
                SELECT c.* FROM observation_conflicts c
                JOIN observations o ON o.id = c.left_observation_id
                WHERE c.status = 'open' AND o.project_id = $1
                ORDER BY c.created_at DESC
                "#,
            )
            .bind(pid)
            .fetch_all(&self.pool)
            .await
            ?
        } else {
            sqlx::query(
                r#"
                SELECT * FROM observation_conflicts
                WHERE status = 'open'
                ORDER BY created_at DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await
            ?
        };

        Ok(rows.iter().map(row_to_conflict).collect())
    }

    pub async fn resolve_conflict(
        &self,
        conflict_id: Uuid,
        status: &str,
    ) -> Result<(), MemoryError> {
        sqlx::query(
            r#"
            UPDATE observation_conflicts
            SET status = $2, resolved_at = now()
            WHERE id = $1
            "#,
        )
        .bind(conflict_id)
        .bind(status)
        .execute(&self.pool)
        .await
        ?;

        Ok(())
    }
}

fn row_to_conflict(row: &sqlx::postgres::PgRow) -> ConflictRecord {
    ConflictRecord {
        id: row.get("id"),
        left_observation_id: row.get("left_observation_id"),
        right_observation_id: row.get("right_observation_id"),
        conflict_type: row.get("conflict_type"),
        description: row.get("description"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        resolved_at: row.get("resolved_at"),
    }
}
