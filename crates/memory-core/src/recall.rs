use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::observation::{MemoryConfidence, MemoryKind, MemoryScope, MemoryStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallRequest {
    pub task: String,
    pub scope: MemoryScope,
    pub project_id: Option<Uuid>,
    pub files: Vec<String>,
    pub token_budget: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallResponse {
    pub memories: Vec<RecallMemory>,
    pub token_estimate: usize,
    pub budget_exceeded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallMemory {
    pub id: Uuid,
    pub kind: MemoryKind,
    pub summary: String,
    pub confidence: MemoryConfidence,
    pub status: MemoryStatus,
    pub evidence_count: usize,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub session_start: usize,
    pub task_recall: usize,
    pub file_recall: usize,
    pub architecture_recall: usize,
    pub debugging_recall: usize,
    pub preference_recall: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            session_start: 1000,
            task_recall: 1200,
            file_recall: 700,
            architecture_recall: 1800,
            debugging_recall: 1500,
            preference_recall: 200,
        }
    }
}

impl TokenBudget {
    pub fn for_task(task: &str) -> usize {
        let task_lower = task.to_lowercase();
        if task_lower.contains("debug") || task_lower.contains("bug") || task_lower.contains("fix") {
            1500
        } else if task_lower.contains("architect") || task_lower.contains("design") {
            1800
        } else if task_lower.contains("preference") || task_lower.contains("style") {
            200
        } else {
            1200
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_token_budget() {
        let budget = TokenBudget::default();
        assert_eq!(budget.session_start, 1000);
        assert_eq!(budget.task_recall, 1200);
    }

    #[test]
    fn test_token_budget_for_debug_task() {
        assert_eq!(TokenBudget::for_task("debug failed auth middleware tests"), 1500);
    }

    #[test]
    fn test_token_budget_for_architecture_task() {
        assert_eq!(TokenBudget::for_task("design new architecture"), 1800);
    }

    #[test]
    fn test_token_budget_for_preference_task() {
        assert_eq!(TokenBudget::for_task("user style preferences"), 200);
    }

    #[test]
    fn test_token_budget_default() {
        assert_eq!(TokenBudget::for_task("add new feature"), 1200);
    }
}
