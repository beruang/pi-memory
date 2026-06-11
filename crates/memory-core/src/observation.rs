use serde::{Deserialize, Serialize};
use uuid::Uuid;
use time::OffsetDateTime;

use super::evidence::EvidenceRef;
use super::errors::MemoryError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: Uuid,
    pub scope: MemoryScope,
    pub project_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub session_id: String,

    pub kind: MemoryKind,
    pub summary: String,

    pub entities: Vec<String>,
    pub files: Vec<String>,
    pub commands: Vec<String>,
    pub links: Vec<String>,

    pub confidence: MemoryConfidence,
    pub sensitivity: MemorySensitivity,
    pub status: MemoryStatus,

    pub evidence: Vec<EvidenceRef>,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub last_accessed_at: Option<OffsetDateTime>,
    pub last_confirmed_at: Option<OffsetDateTime>,
    pub valid_until: Option<OffsetDateTime>,

    pub supersedes: Vec<Uuid>,
    pub superseded_by: Option<Uuid>,

    pub metadata: serde_json::Value,
}

impl Observation {
    pub fn new(
        scope: MemoryScope,
        session_id: String,
        kind: MemoryKind,
        summary: String,
        confidence: MemoryConfidence,
        sensitivity: MemorySensitivity,
    ) -> Result<Self, MemoryError> {
        if sensitivity == MemorySensitivity::Secret {
            return Err(MemoryError::SecretContentRejected);
        }

        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: Uuid::new_v4(),
            scope,
            project_id: None,
            user_id: None,
            organization_id: None,
            session_id,
            kind,
            summary,
            entities: vec![],
            files: vec![],
            commands: vec![],
            links: vec![],
            confidence,
            sensitivity,
            status: MemoryStatus::Active,
            evidence: vec![],
            created_at: now,
            updated_at: now,
            last_accessed_at: None,
            last_confirmed_at: None,
            valid_until: None,
            supersedes: vec![],
            superseded_by: None,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        })
    }

    pub fn touch(&mut self) {
        self.last_accessed_at = Some(OffsetDateTime::now_utc());
    }

    pub fn confirm(&mut self) {
        self.last_confirmed_at = Some(OffsetDateTime::now_utc());
        if self.status == MemoryStatus::Unconfirmed {
            self.status = MemoryStatus::Active;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    Session,
    Project,
    User,
    Organization,
}

impl std::fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryScope::Session => write!(f, "session"),
            MemoryScope::Project => write!(f, "project"),
            MemoryScope::User => write!(f, "user"),
            MemoryScope::Organization => write!(f, "organization"),
        }
    }
}

impl std::str::FromStr for MemoryScope {
    type Err = MemoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "session" => Ok(MemoryScope::Session),
            "project" => Ok(MemoryScope::Project),
            "user" => Ok(MemoryScope::User),
            "organization" => Ok(MemoryScope::Organization),
            _ => Err(MemoryError::InvalidScope),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Decision,
    Fact,
    Constraint,
    Preference,
    Procedure,
    ImplementationDetail,
    Bug,
    Fix,
    FailedAttempt,
    Todo,
    OpenQuestion,
    Dependency,
    Risk,
    Policy,
    ExternalReference,
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryKind::Decision => write!(f, "decision"),
            MemoryKind::Fact => write!(f, "fact"),
            MemoryKind::Constraint => write!(f, "constraint"),
            MemoryKind::Preference => write!(f, "preference"),
            MemoryKind::Procedure => write!(f, "procedure"),
            MemoryKind::ImplementationDetail => write!(f, "implementation_detail"),
            MemoryKind::Bug => write!(f, "bug"),
            MemoryKind::Fix => write!(f, "fix"),
            MemoryKind::FailedAttempt => write!(f, "failed_attempt"),
            MemoryKind::Todo => write!(f, "todo"),
            MemoryKind::OpenQuestion => write!(f, "open_question"),
            MemoryKind::Dependency => write!(f, "dependency"),
            MemoryKind::Risk => write!(f, "risk"),
            MemoryKind::Policy => write!(f, "policy"),
            MemoryKind::ExternalReference => write!(f, "external_reference"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryConfidence {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for MemoryConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryConfidence::Low => write!(f, "low"),
            MemoryConfidence::Medium => write!(f, "medium"),
            MemoryConfidence::High => write!(f, "high"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySensitivity {
    Public,
    Internal,
    Private,
    Secret,
}

impl std::fmt::Display for MemorySensitivity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemorySensitivity::Public => write!(f, "public"),
            MemorySensitivity::Internal => write!(f, "internal"),
            MemorySensitivity::Private => write!(f, "private"),
            MemorySensitivity::Secret => write!(f, "secret"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Active,
    Unconfirmed,
    Superseded,
    Obsolete,
    Conflicted,
    Deleted,
}

impl std::fmt::Display for MemoryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryStatus::Active => write!(f, "active"),
            MemoryStatus::Unconfirmed => write!(f, "unconfirmed"),
            MemoryStatus::Superseded => write!(f, "superseded"),
            MemoryStatus::Obsolete => write!(f, "obsolete"),
            MemoryStatus::Conflicted => write!(f, "conflicted"),
            MemoryStatus::Deleted => write!(f, "deleted"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_observation_rejects_secret() {
        let result = Observation::new(
            MemoryScope::Project,
            "session-1".into(),
            MemoryKind::Fact,
            "test".into(),
            MemoryConfidence::Medium,
            MemorySensitivity::Secret,
        );
        assert!(matches!(result, Err(MemoryError::SecretContentRejected)));
    }

    #[test]
    fn test_new_observation_creates_active() {
        let obs = Observation::new(
            MemoryScope::Project,
            "session-1".into(),
            MemoryKind::Fact,
            "test summary".into(),
            MemoryConfidence::High,
            MemorySensitivity::Internal,
        )
        .unwrap();
        assert_eq!(obs.status, MemoryStatus::Active);
        assert_eq!(obs.confidence, MemoryConfidence::High);
        assert_eq!(obs.summary, "test summary");
        assert!(!obs.id.is_nil());
    }

    #[test]
    fn test_scope_display_and_parse() {
        assert_eq!(MemoryScope::Project.to_string(), "project");
        assert_eq!("user".parse::<MemoryScope>().unwrap(), MemoryScope::User);
        assert!("invalid".parse::<MemoryScope>().is_err());
    }

    #[test]
    fn test_kind_display() {
        assert_eq!(MemoryKind::FailedAttempt.to_string(), "failed_attempt");
        assert_eq!(MemoryKind::ImplementationDetail.to_string(), "implementation_detail");
    }

    #[test]
    fn test_confirm_transitions_unconfirmed_to_active() {
        let mut obs = Observation::new(
            MemoryScope::Project,
            "s1".into(),
            MemoryKind::Fact,
            "t".into(),
            MemoryConfidence::Low,
            MemorySensitivity::Internal,
        )
        .unwrap();
        obs.status = MemoryStatus::Unconfirmed;
        obs.confirm();
        assert_eq!(obs.status, MemoryStatus::Active);
        assert!(obs.last_confirmed_at.is_some());
    }

    #[test]
    fn test_touch_sets_accessed_at() {
        let mut obs = Observation::new(
            MemoryScope::Project,
            "s1".into(),
            MemoryKind::Fact,
            "t".into(),
            MemoryConfidence::Medium,
            MemorySensitivity::Public,
        )
        .unwrap();
        assert!(obs.last_accessed_at.is_none());
        obs.touch();
        assert!(obs.last_accessed_at.is_some());
    }
}
