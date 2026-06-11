# Spec Phase 3: Provider Abstractions and Implementations

## Phase Goal

Implement `memory-providers` with concrete implementations of the `EmbeddingProvider` and `ConsolidationProvider` traits defined in `memory-core`, plus Context7 integration for documentation lookup.

## Dependencies

- Requires: Phase 1 (EmbeddingProvider and ConsolidationProvider traits from memory-core)
- Produces: `crates/memory-providers/`

## Existing Code References

- Pattern to follow: `#[async_trait::async_trait]` trait implementations with reqwest HTTP client.
- Test pattern: Mock providers for unit tests, feature-gated integration tests for real providers.
- Config pattern: API keys from environment variables, model selection from config.
- Reference: `spec.md` sections 6 (Provider traits), 25 (Context7), 33 (Embedding Policy).

## Technical Approach

Implement each provider as a struct with an async `new()` constructor that validates configuration. Use `reqwest::Client` with timeouts and retries. Embedding providers call their respective APIs and return `Vec<f32>`. Consolidation providers receive `ConsolidationInput` and return `Vec<CandidateObservation>`.

**Key design decisions:**
- Provider selection is via config string (e.g., `embedding.provider = "openai"`), not compile-time features.
- Each provider is optional via Cargo features (`openai`, `ollama`, `anthropic`).
- Mock providers are always available for testing.
- Context7 is treated as a documentation lookup tool, not a memory provider.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/memory-providers/Cargo.toml` | reqwest, serde, async-trait, memory-core, feature flags |
| `crates/memory-providers/src/lib.rs` | Provider registry, factory function |
| `crates/memory-providers/src/embeddings.rs` | OpenAI, Ollama, and mock embedding providers |
| `crates/memory-providers/src/consolidation.rs` | Anthropic and mock consolidation providers |
| `crates/memory-providers/src/context7.rs` | Context7 API client for doc lookup |

### Modified Files

- `Cargo.toml` (workspace root) — add `memory-providers` to workspace members.

## Implementation Steps

1. Create `crates/memory-providers/Cargo.toml` with reqwest, serde, async-trait, memory-core. Feature flags: `openai`, `ollama`, `anthropic`.
2. Implement `src/lib.rs` — `ProviderRegistry` with factory functions: `create_embedding_provider(kind: &str, config: &ProviderConfig) -> Box<dyn EmbeddingProvider>`, same for consolidation.
3. Implement `src/embeddings.rs`:
   - `OpenAiEmbeddingProvider` — calls `https://api.openai.com/v1/embeddings`, model configurable (default: `text-embedding-3-small`).
   - `OllamaEmbeddingProvider` — calls local Ollama API, model configurable.
   - `MockEmbeddingProvider` — returns deterministic vectors for testing.
4. Implement `src/consolidation.rs`:
   - `AnthropicConsolidationProvider` — sends session events + existing memory + prompt, receives candidate observations.
   - `MockConsolidationProvider` — returns pre-configured candidates for testing.
5. Implement `src/context7.rs` — Context7 client wrapping REST API for library documentation queries.

## Data / API / Interface Contract

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,       // from env var, not hardcoded
    pub model: String,
    pub base_url: Option<String>,      // for self-hosted/Ollama
    pub dimensions: Option<usize>,
}

pub fn create_embedding_provider(
    kind: &str,
    config: &ProviderConfig,
) -> Result<Box<dyn EmbeddingProvider>, MemoryError>;

pub fn create_consolidation_provider(
    kind: &str,
    config: &ProviderConfig,
) -> Result<Box<dyn ConsolidationProvider>, MemoryError>;

impl EmbeddingProvider for OpenAiEmbeddingProvider {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, MemoryError>;
}

impl ConsolidationProvider for AnthropicConsolidationProvider {
    async fn consolidate(
        &self,
        input: ConsolidationInput,
    ) -> Result<Vec<CandidateObservation>, MemoryError>;
}
```

## Error Handling

- `MemoryError::EmbeddingProvider(String)` — provider API errors, rate limits, invalid responses.
- `MemoryError::ConsolidationProvider(String)` — consolidation failures.
- API errors mapped to MemoryError variants with safe messages (no API keys in errors).

## Observability

- Logs: `tracing::info` for provider initialization, `tracing::debug` for API call latency.
- Metrics: embedding latency histogram, consolidation latency histogram, API error count.
- Traces: spans around external API calls.

## Testing Requirements

### Unit Tests

- MockEmbeddingProvider returns known vector for given input.
- MockConsolidationProvider returns known candidates.
- Provider factory returns correct provider type for each kind string.
- Provider factory returns error for unknown provider kind.

### Integration Tests

- OpenAiEmbeddingProvider (feature-gated, requires `OPENAI_API_KEY` env var).
- AnthropicConsolidationProvider (feature-gated, requires `ANTHROPIC_API_KEY` env var).
- Context7 lookup for a known library (e.g., "sqlx").

### Regression Tests

None — provider behavior is inherently API-dependent.

## Validation Commands

```bash
cargo build -p memory-providers --all-features
cargo test -p memory-providers
cargo test -p memory-providers --features openai -- --ignored  # real API tests
cargo clippy -p memory-providers --all-features -- -D warnings
```

## Acceptance Criteria

- [ ] `create_embedding_provider("openai", ...)` returns an OpenAI embedding provider.
- [ ] `create_embedding_provider("mock", ...)` returns a mock provider for testing.
- [ ] `create_consolidation_provider("anthropic", ...)` returns an Anthropic consolidation provider.
- [ ] Mock providers work without network access.
- [ ] API keys are read from environment variables, never hardcoded.
- [ ] Provider errors are mapped to `MemoryError` without leaking credentials.
- [ ] Context7 client can fetch documentation for a known library.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Provider API changes break integration | medium | Pin API versions; use feature-gated integration tests with automated runs |
| API key leakage in logs or errors | high | Strip Authorization headers from error messages; use tracing filters |
| Embedding dimension mismatch with database | high | Validate dimension from API response against configured value; reject on mismatch |
