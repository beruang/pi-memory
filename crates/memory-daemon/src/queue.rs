use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: JobType,
    pub payload: serde_json::Value,
    pub attempts: u32,
    pub max_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    ConsolidateSession,
    GenerateEmbedding,
    CleanupEvents,
    DetectConflicts,
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobType::ConsolidateSession => write!(f, "consolidate_session"),
            JobType::GenerateEmbedding => write!(f, "generate_embedding"),
            JobType::CleanupEvents => write!(f, "cleanup_events"),
            JobType::DetectConflicts => write!(f, "detect_conflicts"),
        }
    }
}

pub struct JobQueue {
    queue: Mutex<VecDeque<Job>>,
    dead_letter: Mutex<Vec<Job>>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            dead_letter: Mutex::new(Vec::new()),
        }
    }

    pub async fn enqueue(&self, job: Job) {
        self.queue.lock().await.push_back(job);
    }

    pub async fn dequeue(&self) -> Option<Job> {
        self.queue.lock().await.pop_front()
    }

    pub async fn nack(&self, mut job: Job) {
        job.attempts += 1;
        if job.attempts >= job.max_attempts {
            self.dead_letter.lock().await.push(job);
        } else {
            self.queue.lock().await.push_back(job);
        }
    }

    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }

    pub async fn dead_letter_count(&self) -> usize {
        self.dead_letter.lock().await.len()
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let queue = JobQueue::new();
        let job = Job {
            id: Uuid::new_v4(),
            job_type: JobType::ConsolidateSession,
            payload: serde_json::json!({"session_id": "s1"}),
            attempts: 0,
            max_attempts: 3,
        };
        queue.enqueue(job).await;
        assert_eq!(queue.len().await, 1);
        let dequeued = queue.dequeue().await;
        assert!(dequeued.is_some());
        assert_eq!(queue.len().await, 0);
    }

    #[tokio::test]
    async fn test_nack_to_dead_letter() {
        let queue = JobQueue::new();
        let job = Job {
            id: Uuid::new_v4(),
            job_type: JobType::GenerateEmbedding,
            payload: serde_json::json!({}),
            attempts: 2,
            max_attempts: 3,
        };
        queue.nack(job).await;
        assert_eq!(queue.dead_letter_count().await, 1);
    }

    #[tokio::test]
    async fn test_nack_retry() {
        let queue = JobQueue::new();
        let job = Job {
            id: Uuid::new_v4(),
            job_type: JobType::CleanupEvents,
            payload: serde_json::json!({}),
            attempts: 0,
            max_attempts: 3,
        };
        queue.nack(job).await;
        assert_eq!(queue.len().await, 1);
        assert_eq!(queue.dead_letter_count().await, 0);
    }
}
