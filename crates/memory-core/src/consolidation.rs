use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::errors::MemoryError;
use super::event::SessionEvent;
use super::observation::{MemoryConfidence, MemoryKind, MemoryScope, MemorySensitivity, Observation};

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, MemoryError>;
}

#[async_trait]
pub trait ConsolidationProvider: Send + Sync {
    async fn consolidate(
        &self,
        input: ConsolidationInput,
    ) -> Result<Vec<CandidateObservation>, MemoryError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationInput {
    pub session_id: String,
    pub project_id: Option<uuid::Uuid>,
    pub events: Vec<SessionEvent>,
    pub existing_observations: Vec<Observation>,
    pub user_instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationOutput {
    pub candidates: Vec<CandidateObservation>,
    pub events_processed: usize,
    pub candidates_generated: usize,
    pub conflicts_detected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateObservation {
    pub scope: MemoryScope,
    pub kind: MemoryKind,
    pub summary: String,
    pub confidence: MemoryConfidence,
    pub sensitivity: MemorySensitivity,
    pub entities: Vec<String>,
    pub files: Vec<String>,
    pub commands: Vec<String>,
    pub rationale: Option<String>,
    pub source_event_ids: Vec<uuid::Uuid>,
}

impl CandidateObservation {
    pub fn new(
        scope: MemoryScope,
        kind: MemoryKind,
        summary: String,
        confidence: MemoryConfidence,
        sensitivity: MemorySensitivity,
    ) -> Self {
        Self {
            scope,
            kind,
            summary,
            confidence,
            sensitivity,
            entities: vec![],
            files: vec![],
            commands: vec![],
            rationale: None,
            source_event_ids: vec![],
        }
    }
}
