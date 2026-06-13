use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("observation not found: {0}")]
    ObservationNotFound(uuid::Uuid),

    #[error("invalid memory scope")]
    InvalidScope,

    #[error("invalid id: {0}")]
    InvalidId(String),

    #[error("secret content cannot be persisted")]
    SecretContentRejected,

    #[error("conflicting memory detected")]
    ConflictDetected,

    #[error("invalid status transition from {from:?} to {to:?}")]
    InvalidStatusTransition { from: String, to: String },

    #[error("evidence is required for durable memory")]
    MissingEvidence,

    #[error("embedding provider error: {0}")]
    EmbeddingProvider(String),

    #[error("consolidation provider error: {0}")]
    ConsolidationProvider(String),

    #[cfg_attr(feature = "db", error("{0}"))]
    #[cfg_attr(not(feature = "db"), error("database error: {0}"))]
    Database(
        #[cfg(feature = "db")]
        #[from]
        sqlx::Error,
        #[cfg(not(feature = "db"))]
        String,
    ),

    #[error("authorization denied")]
    AuthorizationDenied,
}
