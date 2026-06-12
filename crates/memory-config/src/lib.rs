use figment::providers::{Format, Toml};
use figment::Figment;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    #[serde(default)]
    pub default_scope: String,
    #[serde(default = "default_event_retention_days")]
    pub event_retention_days: u32,
    #[serde(default = "default_max_recall_tokens")]
    pub max_recall_tokens: usize,
    #[serde(default = "default_true")]
    pub enable_private_blocks: bool,
    #[serde(default = "default_true")]
    pub enable_secret_scanner: bool,
    pub embedding: EmbeddingConfig,
    #[serde(default = "default_mcp_transport")]
    pub mcp_transport: String,
    #[serde(default)]
    pub api: ApiConfig,
}

fn default_event_retention_days() -> u32 {
    30
}
fn default_max_recall_tokens() -> usize {
    1200
}
fn default_true() -> bool {
    true
}
fn default_mcp_transport() -> String {
    "stdio".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_embedding_kind")]
    pub kind: String,
    #[serde(default = "default_embedding_model")]
    pub model: String,
    #[serde(default = "default_embedding_dimensions")]
    pub dimensions: usize,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
}

fn default_embedding_kind() -> String {
    "mock".to_string()
}
fn default_embedding_model() -> String {
    "mock".to_string()
}
fn default_embedding_dimensions() -> usize {
    1536
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_api_host")]
    pub host: String,
    #[serde(default = "default_api_port")]
    pub port: u16,
}

fn default_api_host() -> String {
    "0.0.0.0".to_string()
}
fn default_api_port() -> u16 {
    8080
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            kind: default_embedding_kind(),
            model: default_embedding_model(),
            dimensions: default_embedding_dimensions(),
            base_url: None,
            api_key_env: None,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: default_api_host(),
            port: default_api_port(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost:5432/agent_memory".into()),
            default_scope: "project".to_string(),
            event_retention_days: default_event_retention_days(),
            max_recall_tokens: default_max_recall_tokens(),
            enable_private_blocks: default_true(),
            enable_secret_scanner: default_true(),
            embedding: EmbeddingConfig::default(),
            mcp_transport: default_mcp_transport(),
            api: ApiConfig::default(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("figment error")]
    Figment,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl AppConfig {
    /// Load configuration from a TOML file and environment variables.
    /// Env prefix: AGENT_MEMORY__
    /// Default config path: ./agent-memory.toml
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        let figment = Figment::new()
            .merge(figment::providers::Env::prefixed("AGENT_MEMORY__"))
            .merge(Toml::file(
                path.unwrap_or_else(|| Path::new("agent-memory.toml")),
            ));

        Ok(figment.extract().unwrap_or_else(|_| AppConfig::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.default_scope, "project");
        assert_eq!(config.event_retention_days, 30);
        assert_eq!(config.embedding.dimensions, 1536);
    }
}
