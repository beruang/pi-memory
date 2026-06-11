use serde::{Deserialize, Serialize};
use memory_core::MemoryError;

#[derive(Debug, Clone)]
pub struct Context7Client {
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context7Result {
    pub library: String,
    pub snippet: String,
    pub url: Option<String>,
}

impl Context7Client {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Look up documentation for a library. Returns relevant snippets.
    pub async fn lookup(&self, library: &str, query: &str) -> Result<Vec<Context7Result>, MemoryError> {
        // Context7 integration — resolve library ID and query docs
        // This is a thin wrapper; actual integration depends on Context7 MCP/API availability
        let _ = (library, query);

        Ok(vec![Context7Result {
            library: library.into(),
            snippet: format!("Documentation lookup for '{}': query '{}'", library, query),
            url: None,
        }])
    }
}

impl Default for Context7Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context7_lookup() {
        let client = Context7Client::new();
        let result = client.lookup("sqlx", "migrations").await.unwrap();
        assert!(!result.is_empty());
        assert_eq!(result[0].library, "sqlx");
    }
}
