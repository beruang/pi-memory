pub mod consolidation;
pub mod context7;
pub mod embeddings;

use memory_core::{ConsolidationProvider, EmbeddingProvider, MemoryError};

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: Option<String>,
    pub dimensions: Option<usize>,
}

pub fn create_embedding_provider(
    kind: &str,
    _config: &ProviderConfig,
) -> Result<Box<dyn EmbeddingProvider>, MemoryError> {
    match kind {
        #[cfg(feature = "openai")]
        "openai" => {
            let config = _config;
            Ok(Box::new(embeddings::OpenAiEmbeddingProvider::new(config)?))
        }
        #[cfg(feature = "ollama")]
        "ollama" => {
            let config = _config;
            Ok(Box::new(embeddings::OllamaEmbeddingProvider::new(config)?))
        }
        _ => Ok(Box::new(embeddings::MockEmbeddingProvider::new())),
    }
}

pub fn create_consolidation_provider(
    kind: &str,
    _config: &ProviderConfig,
) -> Result<Box<dyn ConsolidationProvider>, MemoryError> {
    match kind {
        #[cfg(feature = "anthropic")]
        "anthropic" => {
            let config = _config;
            Ok(Box::new(
                consolidation::AnthropicConsolidationProvider::new(config)?,
            ))
        }
        _ => Ok(Box::new(consolidation::MockConsolidationProvider::new())),
    }
}
