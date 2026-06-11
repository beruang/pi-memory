use memory_providers::{create_consolidation_provider, create_embedding_provider, ProviderConfig};
use tracing::{error, info};

use super::queue::{Job, JobQueue, JobType};

pub struct ConsolidationWorker {
    queue: std::sync::Arc<JobQueue>,
}

impl ConsolidationWorker {
    pub fn new(queue: std::sync::Arc<JobQueue>) -> Self {
        Self { queue }
    }

    pub async fn run(&self) {
        info!("Consolidation worker started");
        loop {
            if let Some(job) = self.queue.dequeue().await {
                if matches!(job.job_type, JobType::ConsolidateSession) {
                    match self.process_consolidation(&job).await {
                        Ok(_) => info!(job_id = %job.id, "Consolidation job completed"),
                        Err(e) => {
                            error!(job_id = %job.id, error = %e, "Consolidation job failed");
                            self.queue.nack(job).await;
                        }
                    }
                } else {
                    self.queue.enqueue(job).await; // Re-queue non-matching jobs
                }
            }
        }
    }

    async fn process_consolidation(&self, job: &Job) -> Result<(), String> {
        let session_id = job.payload["session_id"]
            .as_str()
            .ok_or("missing session_id")?;

        let provider = create_consolidation_provider("mock", &ProviderConfig {
            api_key: None,
            model: "mock".into(),
            base_url: None,
            dimensions: None,
        })
        .map_err(|e| e.to_string())?;

        let input = memory_core::ConsolidationInput {
            session_id: session_id.to_string(),
            project_id: None,
            events: vec![],
            existing_observations: vec![],
            user_instructions: None,
        };

        let _candidates = provider.consolidate(input).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct EmbeddingWorker {
    queue: std::sync::Arc<JobQueue>,
}

impl EmbeddingWorker {
    pub fn new(queue: std::sync::Arc<JobQueue>) -> Self {
        Self { queue }
    }

    pub async fn run(&self) {
        info!("Embedding worker started");
        loop {
            if let Some(job) = self.queue.dequeue().await {
                if matches!(job.job_type, JobType::GenerateEmbedding) {
                    match self.process_embedding(&job).await {
                        Ok(_) => info!(job_id = %job.id, "Embedding job completed"),
                        Err(e) => {
                            error!(job_id = %job.id, error = %e, "Embedding job failed");
                            self.queue.nack(job).await;
                        }
                    }
                } else {
                    self.queue.enqueue(job).await;
                }
            }
        }
    }

    async fn process_embedding(&self, job: &Job) -> Result<(), String> {
        let text = job.payload["text"].as_str().ok_or("missing text")?;
        let provider = create_embedding_provider("mock", &ProviderConfig {
            api_key: None,
            model: "mock".into(),
            base_url: None,
            dimensions: Some(128),
        })
        .map_err(|e| e.to_string())?;

        let _embedding = provider.embed(text).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
