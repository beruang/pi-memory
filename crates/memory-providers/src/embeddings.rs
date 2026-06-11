use async_trait::async_trait;
use memory_core::{EmbeddingProvider, MemoryError};

pub struct MockEmbeddingProvider;

impl MockEmbeddingProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockEmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, MemoryError> {
        // Deterministic mock: hash-like behavior from input length
        let len = input.len().min(128);
        let mut vec = Vec::with_capacity(len);
        for (i, byte) in input.bytes().take(len).enumerate() {
            vec.push((byte as f32) / 255.0 + (i as f32 * 0.01));
        }
        // Pad or truncate to fixed dimension
        vec.resize(128, 0.0);
        Ok(vec)
    }
}

#[cfg(feature = "openai")]
pub struct OpenAiEmbeddingProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[cfg(feature = "openai")]
impl OpenAiEmbeddingProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, MemoryError> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| MemoryError::EmbeddingProvider("OPENAI_API_KEY not set".into()))?;

        Ok(Self {
            api_key,
            model: config.model.clone(),
            client: reqwest::Client::new(),
        })
    }
}

#[cfg(feature = "openai")]
#[async_trait]
impl EmbeddingProvider for OpenAiEmbeddingProvider {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, MemoryError> {
        let response: serde_json::Value = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "input": input,
                "model": self.model,
            }))
            .send()
            .await
            .map_err(|e| MemoryError::EmbeddingProvider(e.to_string()))?
            .json()
            .await
            .map_err(|e| MemoryError::EmbeddingProvider(e.to_string()))?;

        let embedding = response["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| MemoryError::EmbeddingProvider("invalid response format".into()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }
}

#[cfg(feature = "ollama")]
pub struct OllamaEmbeddingProvider {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

#[cfg(feature = "ollama")]
impl OllamaEmbeddingProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, MemoryError> {
        Ok(Self {
            base_url: config.base_url.clone().unwrap_or_else(|| "http://localhost:11434".into()),
            model: config.model.clone(),
            client: reqwest::Client::new(),
        })
    }
}

#[cfg(feature = "ollama")]
#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, MemoryError> {
        let response: serde_json::Value = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": input,
            }))
            .send()
            .await
            .map_err(|e| MemoryError::EmbeddingProvider(e.to_string()))?
            .json()
            .await
            .map_err(|e| MemoryError::EmbeddingProvider(e.to_string()))?;

        let embedding = response["embedding"]
            .as_array()
            .ok_or_else(|| MemoryError::EmbeddingProvider("invalid response format".into()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_provider() {
        let provider = MockEmbeddingProvider::new();
        let result = provider.embed("test input").await.unwrap();
        assert_eq!(result.len(), 128);
        assert!(result.iter().any(|&v| v != 0.0));
    }

    #[tokio::test]
    async fn test_mock_embedding_deterministic() {
        let provider = MockEmbeddingProvider::new();
        let a = provider.embed("hello").await.unwrap();
        let b = provider.embed("hello").await.unwrap();
        assert_eq!(a, b);
    }
}
