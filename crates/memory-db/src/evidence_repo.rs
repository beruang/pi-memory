use sqlx::{PgPool, Row};
use uuid::Uuid;

use memory_core::{EvidenceRef, EvidenceSourceType, MemoryError};

pub struct EvidenceRepository {
    pool: PgPool,
}

impl EvidenceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, evidence: &EvidenceRef) -> Result<EvidenceRef, MemoryError> {
        sqlx::query(
            r#"
            INSERT INTO evidence (id, observation_id, source_type, source_id, source_location, excerpt, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(evidence.id)
        .bind(evidence.observation_id)
        .bind(evidence.source_type.to_string())
        .bind(&evidence.source_id)
        .bind(evidence.source_location.as_deref())
        .bind(evidence.excerpt.as_deref())
        .bind(evidence.created_at)
        .execute(&self.pool)
        .await
        ?;

        Ok(evidence.clone())
    }

    pub async fn get_by_observation_id(
        &self,
        observation_id: Uuid,
    ) -> Result<Vec<EvidenceRef>, MemoryError> {
        let rows =
            sqlx::query(r#"SELECT * FROM evidence WHERE observation_id = $1 ORDER BY created_at"#)
                .bind(observation_id)
                .fetch_all(&self.pool)
                .await
                ?;

        Ok(rows.iter().map(row_to_evidence).collect())
    }

    pub async fn delete_by_observation_id(&self, observation_id: Uuid) -> Result<(), MemoryError> {
        sqlx::query(r#"DELETE FROM evidence WHERE observation_id = $1"#)
            .bind(observation_id)
            .execute(&self.pool)
            .await
            ?;

        Ok(())
    }
}

fn row_to_evidence(row: &sqlx::postgres::PgRow) -> EvidenceRef {
    let source_type_str: String = row.get("source_type");
    EvidenceRef {
        id: row.get("id"),
        observation_id: row.get("observation_id"),
        source_type: source_type_from_str(&source_type_str),
        source_id: row.get("source_id"),
        source_location: row.get("source_location"),
        excerpt: row.get("excerpt"),
        created_at: row.get("created_at"),
    }
}

fn source_type_from_str(s: &str) -> EvidenceSourceType {
    match s {
        "message" => EvidenceSourceType::Message,
        "tool_call" => EvidenceSourceType::ToolCall,
        "file" => EvidenceSourceType::File,
        "terminal" => EvidenceSourceType::Terminal,
        "commit" => EvidenceSourceType::Commit,
        "issue" => EvidenceSourceType::Issue,
        "pull_request" => EvidenceSourceType::PullRequest,
        "document" => EvidenceSourceType::Document,
        "web" => EvidenceSourceType::Web,
        "manual_entry" => EvidenceSourceType::ManualEntry,
        _ => EvidenceSourceType::ManualEntry,
    }
}
