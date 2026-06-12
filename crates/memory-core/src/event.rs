use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::observation::MemorySensitivity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub id: Uuid,
    pub session_id: String,
    pub project_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub sensitivity: MemorySensitivity,
    pub redacted: bool,
    pub created_at: OffsetDateTime,
}

impl SessionEvent {
    pub fn new(session_id: String, event_type: String, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            project_id: None,
            user_id: None,
            organization_id: None,
            event_type,
            payload,
            sensitivity: MemorySensitivity::Internal,
            redacted: false,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn mark_redacted(&mut self) {
        self.redacted = true;
    }

    pub fn with_sensitivity(mut self, sensitivity: MemorySensitivity) -> Self {
        self.sensitivity = sensitivity;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    UserPrompt,
    AssistantResponse,
    FileRead,
    FileWrite,
    ShellCommand,
    TerminalOutput,
    ErrorOutput,
    TestResult,
    CodeDiff,
    CommitMetadata,
    IssueReference,
    PullRequestReference,
    ExplicitMemoryCommand,
    SessionStart,
    SessionEnd,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::UserPrompt => write!(f, "user_prompt"),
            EventType::AssistantResponse => write!(f, "assistant_response"),
            EventType::FileRead => write!(f, "file_read"),
            EventType::FileWrite => write!(f, "file_write"),
            EventType::ShellCommand => write!(f, "shell_command"),
            EventType::TerminalOutput => write!(f, "terminal_output"),
            EventType::ErrorOutput => write!(f, "error_output"),
            EventType::TestResult => write!(f, "test_result"),
            EventType::CodeDiff => write!(f, "code_diff"),
            EventType::CommitMetadata => write!(f, "commit_metadata"),
            EventType::IssueReference => write!(f, "issue_reference"),
            EventType::PullRequestReference => write!(f, "pull_request_reference"),
            EventType::ExplicitMemoryCommand => write!(f, "explicit_memory_command"),
            EventType::SessionStart => write!(f, "session_start"),
            EventType::SessionEnd => write!(f, "session_end"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_event_creation() {
        let payload = serde_json::json!({"text": "hello"});
        let event = SessionEvent::new("s1".into(), "user_prompt".into(), payload);
        assert_eq!(event.session_id, "s1");
        assert!(!event.redacted);
        assert_eq!(event.sensitivity, MemorySensitivity::Internal);
    }

    #[test]
    fn test_mark_redacted() {
        let mut event = SessionEvent::new("s1".into(), "user_prompt".into(), serde_json::json!({}));
        event.mark_redacted();
        assert!(event.redacted);
    }
}
