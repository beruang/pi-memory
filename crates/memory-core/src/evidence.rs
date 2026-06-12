use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub id: Uuid,
    pub observation_id: Uuid,
    pub source_type: EvidenceSourceType,
    pub source_id: String,
    pub source_location: Option<String>,
    pub excerpt: Option<String>,
    pub created_at: OffsetDateTime,
}

impl EvidenceRef {
    pub fn new(observation_id: Uuid, source_type: EvidenceSourceType, source_id: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            observation_id,
            source_type,
            source_id,
            source_location: None,
            excerpt: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn with_excerpt(mut self, excerpt: String) -> Self {
        self.excerpt = Some(excerpt);
        self
    }

    pub fn with_location(mut self, location: String) -> Self {
        self.source_location = Some(location);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceType {
    Message,
    ToolCall,
    File,
    Terminal,
    Commit,
    Issue,
    PullRequest,
    Document,
    Web,
    ManualEntry,
}

impl std::fmt::Display for EvidenceSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceSourceType::Message => write!(f, "message"),
            EvidenceSourceType::ToolCall => write!(f, "tool_call"),
            EvidenceSourceType::File => write!(f, "file"),
            EvidenceSourceType::Terminal => write!(f, "terminal"),
            EvidenceSourceType::Commit => write!(f, "commit"),
            EvidenceSourceType::Issue => write!(f, "issue"),
            EvidenceSourceType::PullRequest => write!(f, "pull_request"),
            EvidenceSourceType::Document => write!(f, "document"),
            EvidenceSourceType::Web => write!(f, "web"),
            EvidenceSourceType::ManualEntry => write!(f, "manual_entry"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_ref_creation() {
        let obs_id = Uuid::new_v4();
        let ev = EvidenceRef::new(obs_id, EvidenceSourceType::Message, "msg-1".into())
            .with_excerpt("Let's use advisory locks here.".into())
            .with_location("session-1/messages/msg-1".into());

        assert_eq!(ev.observation_id, obs_id);
        assert_eq!(ev.source_type, EvidenceSourceType::Message);
        assert_eq!(ev.source_id, "msg-1");
        assert_eq!(
            ev.excerpt.as_deref(),
            Some("Let's use advisory locks here.")
        );
        assert_eq!(
            ev.source_location.as_deref(),
            Some("session-1/messages/msg-1")
        );
    }

    #[test]
    fn test_source_type_display() {
        assert_eq!(EvidenceSourceType::Message.to_string(), "message");
        assert_eq!(EvidenceSourceType::ManualEntry.to_string(), "manual_entry");
    }
}
