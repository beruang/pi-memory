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
        .await?;

        Ok(row_to_observation(&row))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Observation>, MemoryError> {
        let row = sqlx::query(r#"SELECT * FROM observations WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            ?;

        Ok(row.as_ref().map(row_to_observation))
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
        ?
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
        ?;

        Ok(rows.iter().map(row_to_observation).collect())
    }

    pub async fn soft_delete(&self, id: Uuid) -> Result<(), MemoryError> {
        let result = sqlx::query(
            r#"UPDATE observations SET status = 'deleted', updated_at = now() WHERE id = $1"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        ?;

        if result.rows_affected() == 0 {
            return Err(MemoryError::ObservationNotFound(id));
        }
        Ok(())
    }

    /// Permanently remove an observation and all its linked data.
    pub async fn hard_delete(&self, id: Uuid) -> Result<(), MemoryError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(r#"DELETE FROM evidence WHERE observation_id = $1"#)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(r#"DELETE FROM observation_embeddings WHERE observation_id = $1"#)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(r#"DELETE FROM observation_files WHERE observation_id = $1"#)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(r#"DELETE FROM observation_entities WHERE observation_id = $1"#)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(r#"DELETE FROM observation_commands WHERE observation_id = $1"#)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            r#"DELETE FROM observation_supersessions WHERE newer_observation_id = $1 OR older_observation_id = $1"#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            r#"DELETE FROM observation_conflicts WHERE left_observation_id = $1 OR right_observation_id = $1"#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(r#"DELETE FROM observations WHERE id = $1"#)
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Insert observation + all link tables atomically in a transaction.
    pub async fn insert_with_links(
        &self,
        obs: &Observation,
        files: &[String],
        entities: &[String],
        commands: &[String],
        supersedes: &[Uuid],
    ) -> Result<Observation, MemoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            ?;

        // Insert observation
        sqlx::query(
            r#"
            INSERT INTO observations (
                id, scope, project_id, user_id, organization_id, session_id,
                kind, summary, confidence, sensitivity, status,
                valid_until, last_accessed_at, last_confirmed_at,
                superseded_by, metadata, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
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
        .execute(&mut *tx)
        .await
        ?;

        // Insert file links
        for file in files {
            sqlx::query(
                r#"INSERT INTO observation_files (observation_id, file_path) VALUES ($1, $2)"#,
            )
            .bind(obs.id)
            .bind(file)
            .execute(&mut *tx)
            .await
            ?;
        }

        // Insert entity links
        for entity in entities {
            sqlx::query(
                r#"INSERT INTO observation_entities (observation_id, entity) VALUES ($1, $2)"#,
            )
            .bind(obs.id)
            .bind(entity)
            .execute(&mut *tx)
            .await
            ?;
        }

        // Insert command links
        for cmd in commands {
            sqlx::query(
                r#"INSERT INTO observation_commands (observation_id, command) VALUES ($1, $2)"#,
            )
            .bind(obs.id)
            .bind(cmd)
            .execute(&mut *tx)
            .await
            ?;
        }

        // Insert supersession links
        for older_id in supersedes {
            sqlx::query(
                r#"
                INSERT INTO observation_supersessions (newer_observation_id, older_observation_id, reason)
                VALUES ($1, $2, 'superseded')
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(obs.id)
            .bind(older_id)
            .execute(&mut *tx)
            .await
            ?;
        }

        tx.commit()
            .await
            ?;
        Ok(obs.clone())
    }

    /// Fetch observation with files, entities, commands, supersedes populated.
    pub async fn get_by_id_with_links(&self, id: Uuid) -> Result<Option<Observation>, MemoryError> {
        let row = sqlx::query(r#"SELECT * FROM observations WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            ?;

        let Some(row) = row else { return Ok(None) };

        let files: Vec<String> = sqlx::query_scalar(
            r#"SELECT file_path FROM observation_files WHERE observation_id = $1"#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        ?;

        let entities: Vec<String> = sqlx::query_scalar(
            r#"SELECT entity FROM observation_entities WHERE observation_id = $1"#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        ?;

        let commands: Vec<String> = sqlx::query_scalar(
            r#"SELECT command FROM observation_commands WHERE observation_id = $1"#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        ?;

        let supersedes: Vec<Uuid> = sqlx::query_scalar(
            r#"SELECT older_observation_id FROM observation_supersessions WHERE newer_observation_id = $1"#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        ?;

        Ok(Some(row_to_observation_with_links(
            &row, files, entities, commands, supersedes,
        )))
    }

    /// List observations linked to a specific file path.
    pub async fn list_by_file(
        &self,
        file_path: &str,
        limit: i64,
    ) -> Result<Vec<Observation>, MemoryError> {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            r#"SELECT observation_id FROM observation_files WHERE file_path = $1"#,
        )
        .bind(file_path)
        .fetch_all(&self.pool)
        .await
        ?;

        if ids.is_empty() {
            return Ok(vec![]);
        }

        let rows =
            sqlx::query("SELECT * FROM observations WHERE id = ANY($1) AND status != 'deleted'")
                .bind(&ids)
                .fetch_all(&self.pool)
                .await
                ?;

        // Populate links for each row
        let mut observations = Vec::new();
        for row in &rows {
            let id: Uuid = row.get("id");
            let files: Vec<String> = sqlx::query_scalar(
                r#"SELECT file_path FROM observation_files WHERE observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            let entities: Vec<String> = sqlx::query_scalar(
                r#"SELECT entity FROM observation_entities WHERE observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            let commands: Vec<String> = sqlx::query_scalar(
                r#"SELECT command FROM observation_commands WHERE observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            let supersedes: Vec<Uuid> = sqlx::query_scalar(
                r#"SELECT older_observation_id FROM observation_supersessions WHERE newer_observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            observations.push(row_to_observation_with_links(
                row, files, entities, commands, supersedes,
            ));
        }
        let _ = limit; // Use limit in SQL once we switch to ANY(...)
        Ok(observations)
    }

    /// List observations linked to a specific entity.
    pub async fn list_by_entity(
        &self,
        entity: &str,
        limit: i64,
    ) -> Result<Vec<Observation>, MemoryError> {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            r#"SELECT observation_id FROM observation_entities WHERE entity = $1"#,
        )
        .bind(entity)
        .fetch_all(&self.pool)
        .await
        ?;

        if ids.is_empty() {
            return Ok(vec![]);
        }

        let rows =
            sqlx::query("SELECT * FROM observations WHERE id = ANY($1) AND status != 'deleted'")
                .bind(&ids)
                .fetch_all(&self.pool)
                .await
                ?;

        let mut observations = Vec::new();
        for row in &rows {
            let id: Uuid = row.get("id");
            let files: Vec<String> = sqlx::query_scalar(
                r#"SELECT file_path FROM observation_files WHERE observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            let entities: Vec<String> = sqlx::query_scalar(
                r#"SELECT entity FROM observation_entities WHERE observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            let commands: Vec<String> = sqlx::query_scalar(
                r#"SELECT command FROM observation_commands WHERE observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            let supersedes: Vec<Uuid> = sqlx::query_scalar(
                r#"SELECT older_observation_id FROM observation_supersessions WHERE newer_observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            observations.push(row_to_observation_with_links(
                row, files, entities, commands, supersedes,
            ));
        }
        let _ = limit;
        Ok(observations)
    }

    /// List observations with conflicting status for a project (for conflict detection).
    pub async fn list_active_with_entities(
        &self,
        project_id: Option<Uuid>,
    ) -> Result<Vec<Observation>, MemoryError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM observations
            WHERE status = 'active'
              AND ($1::uuid IS NULL OR project_id = $1)
            ORDER BY created_at DESC
            LIMIT 500
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        ?;

        let mut observations = Vec::new();
        for row in &rows {
            let id: Uuid = row.get("id");
            let files: Vec<String> = sqlx::query_scalar(
                r#"SELECT file_path FROM observation_files WHERE observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            let entities: Vec<String> = sqlx::query_scalar(
                r#"SELECT entity FROM observation_entities WHERE observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            let commands: Vec<String> = sqlx::query_scalar(
                r#"SELECT command FROM observation_commands WHERE observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            let supersedes: Vec<Uuid> = sqlx::query_scalar(
                r#"SELECT older_observation_id FROM observation_supersessions WHERE newer_observation_id = $1"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            ?;
            observations.push(row_to_observation_with_links(
                row, files, entities, commands, supersedes,
            ));
        }
        Ok(observations)
    }

    /// Link a file path to an observation.
    pub async fn link_file(
        &self,
        observation_id: Uuid,
        file_path: &str,
    ) -> Result<(), MemoryError> {
        sqlx::query(
            r#"INSERT INTO observation_files (observation_id, file_path) VALUES ($1, $2) ON CONFLICT DO NOTHING"#,
        )
        .bind(observation_id)
        .bind(file_path)
        .execute(&self.pool)
        .await
        ?;
        Ok(())
    }
}

fn row_to_observation(row: &sqlx::postgres::PgRow) -> Observation {
    row_to_observation_with_links(row, vec![], vec![], vec![], vec![])
}

fn row_to_observation_with_links(
    row: &sqlx::postgres::PgRow,
    files: Vec<String>,
    entities: Vec<String>,
    commands: Vec<String>,
    supersedes: Vec<Uuid>,
) -> Observation {
    Observation {
        id: row.get("id"),
        scope: row.get("scope"),
        project_id: row.get("project_id"),
        user_id: row.get("user_id"),
        organization_id: row.get("organization_id"),
        session_id: row.get("session_id"),
        kind: row.get("kind"),
        summary: row.get("summary"),
        entities,
        files,
        commands,
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
        supersedes,
        superseded_by: row.get("superseded_by"),
        metadata: row.get("metadata"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_repo_new() {
        // Verifying construction works
        let pool = PgPool::connect_lazy("postgres://localhost").unwrap();
        let _repo = ObservationsRepository::new(pool);
    }
}
