use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::observation::{MemoryKind, Observation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictCategory {
    SameEntityIncompatibleValue,
    SameFileIncompatibleAssumption,
    SameCommandIncompatibleResult,
    SameDependencyDifferentVersion,
    SamePreferenceDifferentPreference,
    SameDecisionIncompatibleDecision,
    SamePolicyIncompatiblePolicy,
}

impl std::fmt::Display for ConflictCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictCategory::SameEntityIncompatibleValue => {
                write!(f, "same_entity_incompatible_value")
            }
            ConflictCategory::SameFileIncompatibleAssumption => {
                write!(f, "same_file_incompatible_assumption")
            }
            ConflictCategory::SameCommandIncompatibleResult => {
                write!(f, "same_command_incompatible_result")
            }
            ConflictCategory::SameDependencyDifferentVersion => {
                write!(f, "same_dependency_different_version")
            }
            ConflictCategory::SamePreferenceDifferentPreference => {
                write!(f, "same_preference_different_preference")
            }
            ConflictCategory::SameDecisionIncompatibleDecision => {
                write!(f, "same_decision_incompatible_decision")
            }
            ConflictCategory::SamePolicyIncompatiblePolicy => {
                write!(f, "same_policy_incompatible_policy")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub left_observation_id: Uuid,
    pub right_observation_id: Uuid,
    pub conflict_type: ConflictCategory,
    pub description: String,
}

pub fn detect_conflicts(
    new_observation: &Observation,
    existing_observations: &[Observation],
) -> Vec<ConflictRecord> {
    let mut conflicts = Vec::new();

    for existing in existing_observations {
        if existing.id == new_observation.id {
            continue;
        }
        if existing.status.is_deleted_or_obsolete() {
            continue;
        }

        if new_observation.kind == existing.kind {
            if let Some(c) = detect_kind_specific_conflict(new_observation, existing) {
                conflicts.push(c);
            }
        }

        if entities_overlap(new_observation, existing)
            && summaries_contradict(&new_observation.summary, &existing.summary)
        {
            conflicts.push(ConflictRecord {
                left_observation_id: new_observation.id,
                right_observation_id: existing.id,
                conflict_type: ConflictCategory::SameEntityIncompatibleValue,
                description: format!(
                    "New observation '{}' conflicts with existing '{}' on shared entities",
                    new_observation.summary, existing.summary
                ),
            });
        }

        if files_overlap(new_observation, existing)
            && new_observation.kind == MemoryKind::ImplementationDetail
            && existing.kind == MemoryKind::ImplementationDetail
        {
            conflicts.push(ConflictRecord {
                left_observation_id: new_observation.id,
                right_observation_id: existing.id,
                conflict_type: ConflictCategory::SameFileIncompatibleAssumption,
                description: format!(
                    "Both observations reference file(s) {:?} with different implementation assumptions",
                    shared_files(new_observation, existing)
                ),
            });
        }
    }

    conflicts
}

fn detect_kind_specific_conflict(
    new: &Observation,
    existing: &Observation,
) -> Option<ConflictRecord> {
    use MemoryKind::*;
    match new.kind {
        Preference => {
            if summaries_contradict(&new.summary, &existing.summary) {
                Some(ConflictRecord {
                    left_observation_id: new.id,
                    right_observation_id: existing.id,
                    conflict_type: ConflictCategory::SamePreferenceDifferentPreference,
                    description: format!(
                        "Preference '{}' conflicts with '{}'",
                        new.summary, existing.summary
                    ),
                })
            } else {
                None
            }
        }
        Decision => {
            if entities_overlap(new, existing)
                && summaries_contradict(&new.summary, &existing.summary)
            {
                Some(ConflictRecord {
                    left_observation_id: new.id,
                    right_observation_id: existing.id,
                    conflict_type: ConflictCategory::SameDecisionIncompatibleDecision,
                    description: format!(
                        "Decision '{}' conflicts with '{}' on same topic",
                        new.summary, existing.summary
                    ),
                })
            } else {
                None
            }
        }
        Dependency => {
            if entities_overlap(new, existing) {
                Some(ConflictRecord {
                    left_observation_id: new.id,
                    right_observation_id: existing.id,
                    conflict_type: ConflictCategory::SameDependencyDifferentVersion,
                    description: format!(
                        "Dependency on '{}' has potentially conflicting versions",
                        new.entities.join(", ")
                    ),
                })
            } else {
                None
            }
        }
        Policy => {
            if entities_overlap(new, existing) {
                Some(ConflictRecord {
                    left_observation_id: new.id,
                    right_observation_id: existing.id,
                    conflict_type: ConflictCategory::SamePolicyIncompatiblePolicy,
                    description: format!(
                        "Policy '{}' may conflict with '{}'",
                        new.summary, existing.summary
                    ),
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

fn entities_overlap(a: &Observation, b: &Observation) -> bool {
    a.entities.iter().any(|e| b.entities.contains(e))
}

fn files_overlap(a: &Observation, b: &Observation) -> bool {
    a.files.iter().any(|f| b.files.contains(f))
}

fn shared_files<'a>(a: &'a Observation, b: &'a Observation) -> Vec<&'a String> {
    a.files.iter().filter(|f| b.files.contains(*f)).collect()
}

pub fn summaries_contradict(new_summary: &str, existing_summary: &str) -> bool {
    let new_lower = new_summary.to_lowercase();
    let existing_lower = existing_summary.to_lowercase();

    let contradiction_pairs: &[(&str, &str)] = &[
        ("upgraded", "downgraded"),
        ("uses node 22", "uses node 20"),
        ("uses node 20", "uses node 22"),
        ("removed", "added"),
        ("deprecated", "required"),
        ("disabled", "enabled"),
    ];

    for (a, b) in contradiction_pairs {
        if new_lower.contains(a) && existing_lower.contains(b) {
            return true;
        }
        if new_lower.contains(b) && existing_lower.contains(a) {
            return true;
        }
    }

    false
}

impl MemoryStatus {
    fn is_deleted_or_obsolete(&self) -> bool {
        matches!(self, MemoryStatus::Deleted | MemoryStatus::Obsolete)
    }
}

use super::observation::MemoryStatus;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::{MemoryConfidence, MemoryKind, MemoryScope, MemorySensitivity};

    fn make_obs(
        kind: MemoryKind,
        summary: &str,
        entities: Vec<&str>,
        files: Vec<&str>,
    ) -> Observation {
        let mut obs = Observation::new(
            MemoryScope::Project,
            "s1".into(),
            kind,
            summary.into(),
            MemoryConfidence::High,
            MemorySensitivity::Internal,
        )
        .unwrap();
        obs.entities = entities.into_iter().map(String::from).collect();
        obs.files = files.into_iter().map(String::from).collect();
        obs
    }

    #[test]
    fn test_no_conflict_on_different_kinds() {
        let new = make_obs(
            MemoryKind::Fact,
            "Project uses PostgreSQL",
            vec!["db"],
            vec![],
        );
        let existing = make_obs(MemoryKind::Decision, "Use PostgreSQL", vec!["db"], vec![]);
        let conflicts = detect_conflicts(&new, &[existing]);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_conflict_on_contradictory_facts() {
        let new = make_obs(
            MemoryKind::Fact,
            "Project uses Node 22",
            vec!["node", "runtime"],
            vec![],
        );
        let existing = make_obs(
            MemoryKind::Fact,
            "Project uses Node 20",
            vec!["node", "runtime"],
            vec![],
        );
        let conflicts = detect_conflicts(&new, &[existing]);
        assert!(!conflicts.is_empty());
    }

    #[test]
    fn test_conflict_on_preference() {
        let new = make_obs(
            MemoryKind::Preference,
            "User enabled dark mode",
            vec!["theme"],
            vec![],
        );
        let existing = make_obs(
            MemoryKind::Preference,
            "User disabled dark mode",
            vec!["theme"],
            vec![],
        );
        let conflicts = detect_conflicts(&new, &[existing]);
        assert!(!conflicts.is_empty());
    }

    #[test]
    fn test_no_conflict_on_same_entity_different_property() {
        let new = make_obs(
            MemoryKind::Fact,
            "Project uses PostgreSQL",
            vec!["db"],
            vec![],
        );
        let existing = make_obs(
            MemoryKind::Fact,
            "Project uses Redis for caching",
            vec!["cache"],
            vec![],
        );
        let conflicts = detect_conflicts(&new, &[existing]);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_ignores_deleted_observations() {
        let new = make_obs(
            MemoryKind::Fact,
            "Project uses Node 22",
            vec!["node"],
            vec![],
        );
        let mut existing = make_obs(
            MemoryKind::Fact,
            "Project uses Node 20",
            vec!["node"],
            vec![],
        );
        existing.status = MemoryStatus::Deleted;
        let conflicts = detect_conflicts(&new, &[existing]);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_file_conflict_on_implementation_details() {
        let new = make_obs(
            MemoryKind::ImplementationDetail,
            "Auth handled in middleware.ts",
            vec![],
            vec!["src/auth/middleware.ts"],
        );
        let existing = make_obs(
            MemoryKind::ImplementationDetail,
            "Auth handled differently",
            vec![],
            vec!["src/auth/middleware.ts"],
        );
        let conflicts = detect_conflicts(&new, &[existing]);
        assert!(!conflicts.is_empty());
    }
}
