use async_trait::async_trait;
use memory_core::{
    CandidateObservation, ConsolidationInput, ConsolidationProvider, MemoryConfidence, MemoryError,
    MemoryKind, MemoryScope, MemorySensitivity,
};

#[cfg(feature = "anthropic")]
use super::ProviderConfig;

pub struct MockConsolidationProvider;

impl MockConsolidationProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockConsolidationProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConsolidationProvider for MockConsolidationProvider {
    async fn consolidate(
        &self,
        input: ConsolidationInput,
    ) -> Result<Vec<CandidateObservation>, MemoryError> {
        // Return mock candidates based on event types
        let mut candidates = Vec::new();

        for event in &input.events {
            match event.event_type.as_str() {
                "user_prompt" | "assistant_response" => {
                    if let Some(text) = event.payload.get("text").and_then(|v| v.as_str()) {
                        if text.len() > 50 {
                            candidates.push(CandidateObservation {
                                scope: MemoryScope::Project,
                                kind: MemoryKind::Fact,
                                summary: format!(
                                    "Captured from session: {}...",
                                    &text[..50.min(text.len())]
                                ),
                                confidence: MemoryConfidence::Low,
                                sensitivity: MemorySensitivity::Internal,
                                entities: vec![],
                                files: vec![],
                                commands: vec![],
                                rationale: Some("Auto-extracted from session event".into()),
                                source_event_ids: vec![event.id],
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(candidates)
    }
}

#[cfg(feature = "anthropic")]
pub struct AnthropicConsolidationProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[cfg(feature = "anthropic")]
impl AnthropicConsolidationProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, MemoryError> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| {
                MemoryError::ConsolidationProvider("ANTHROPIC_API_KEY not set".into())
            })?;

        Ok(Self {
            api_key,
            model: config.model.clone(),
            client: reqwest::Client::new(),
        })
    }
}

#[cfg(feature = "anthropic")]
#[async_trait]
impl ConsolidationProvider for AnthropicConsolidationProvider {
    async fn consolidate(
        &self,
        input: ConsolidationInput,
    ) -> Result<Vec<CandidateObservation>, MemoryError> {
        let events_json = serde_json::to_string(&input.events)
            .map_err(|e| MemoryError::ConsolidationProvider(e.to_string()))?;

        let prompt = format!(
            "Extract durable observations from these session events. Return a JSON array of observations.\n\nEvents: {}",
            events_json
        );

        let response: serde_json::Value = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": prompt}],
            }))
            .send()
            .await
            .map_err(|e| MemoryError::ConsolidationProvider(e.to_string()))?
            .json()
            .await
            .map_err(|e| MemoryError::ConsolidationProvider(e.to_string()))?;

        // Parse the response - this is simplified; real impl would parse structured output
        let _text = response["content"][0]["text"].as_str().unwrap_or("[]");

        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_core::SessionEvent;

    #[tokio::test]
    async fn test_mock_consolidation_empty() {
        let provider = MockConsolidationProvider::new();
        let input = ConsolidationInput {
            session_id: "s1".into(),
            project_id: None,
            events: vec![],
            existing_observations: vec![],
            user_instructions: None,
        };
        let result = provider.consolidate(input).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_mock_consolidation_with_events() {
        let provider = MockConsolidationProvider::new();
        let event = SessionEvent::new(
            "s1".into(),
            "user_prompt".into(),
            serde_json::json!({"text": "The project needs PostgreSQL advisory locks for job deduplication because Redis locks expired too early during long-running jobs."}),
        );
        let input = ConsolidationInput {
            session_id: "s1".into(),
            project_id: None,
            events: vec![event],
            existing_observations: vec![],
            user_instructions: None,
        };
        let result = provider.consolidate(input).await.unwrap();
        assert!(!result.is_empty());
    }
}
