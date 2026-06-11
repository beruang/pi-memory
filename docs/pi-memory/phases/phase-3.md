# Phase 3: Provider Abstractions and Implementations

**Depends on:** Phase 1 (core traits: EmbeddingProvider, ConsolidationProvider)
**Risk:** medium — external API dependencies; provider behavior can change
**Value:** Enables consolidation and embedding generation behind swappable provider implementations.

## Purpose

Implement `memory-providers` with concrete provider implementations behind the traits defined in `memory-core`. Include Context7 integration for documentation lookup during development.

## Deliverables

- OpenAI embedding provider implementing `EmbeddingProvider` trait.
- Anthropic consolidation provider implementing `ConsolidationProvider` trait.
- Ollama embedding provider (optional, local-first).
- Context7 integration for documentation lookup (wraps Context7 MCP or HTTP API).
- Provider configuration: API keys from env vars, model selection, dimension config.
- Provider factory/registry for selecting provider by configuration.
- Mock providers for testing.

## Files

| File | Purpose |
|------|---------|
| `crates/memory-providers/Cargo.toml` | reqwest, serde, async-trait dependencies |
| `crates/memory-providers/src/lib.rs` | Public re-exports, provider registry |
| `crates/memory-providers/src/embeddings.rs` | OpenAI, Ollama, and mock embedding providers |
| `crates/memory-providers/src/consolidation.rs` | Anthropic and mock consolidation providers |
| `crates/memory-providers/src/context7.rs` | Context7 API integration |

## Testing

- Mock provider unit tests (embedding returns known vector, consolidation returns known candidates).
- Integration tests with real providers (gated behind feature flags or env vars).
- Context7 integration test with a known library query.
