use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::info;

use super::queue::{Job, JobQueue, JobType};

pub struct Scheduler {
    queue: Arc<JobQueue>,
}

impl Scheduler {
    pub fn new(queue: Arc<JobQueue>) -> Self {
        Self { queue }
    }

    pub async fn start(&self) {
        info!("Scheduler started");

        let mut cleanup_timer = interval(Duration::from_secs(3600)); // Every hour
        let mut conflict_timer = interval(Duration::from_secs(1800)); // Every 30 minutes

        loop {
            tokio::select! {
                _ = cleanup_timer.tick() => {
                    self.enqueue_cleanup_job().await;
                }
                _ = conflict_timer.tick() => {
                    self.enqueue_conflict_detection_job().await;
                }
            }
        }
    }

    async fn enqueue_cleanup_job(&self) {
        let job = Job {
            id: uuid::Uuid::new_v4(),
            job_type: JobType::CleanupEvents,
            payload: serde_json::json!({"retention_days": 7}),
            attempts: 0,
            max_attempts: 3,
        };
        self.queue.enqueue(job).await;
        info!("Scheduled cleanup job");
    }

    async fn enqueue_conflict_detection_job(&self) {
        let job = Job {
            id: uuid::Uuid::new_v4(),
            job_type: JobType::DetectConflicts,
            payload: serde_json::json!({}),
            attempts: 0,
            max_attempts: 3,
        };
        self.queue.enqueue(job).await;
        info!("Scheduled conflict detection job");
    }
}
