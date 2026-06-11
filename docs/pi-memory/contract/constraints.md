# Constraints

## File Layout Constraints

- Rust workspace at repository root with `crates/` directory.
- Documentation under `docs/pi-memory/`.
- Agent artifacts under `.agent/contracts/pi-memory/`.
- SQL migrations in `crates/memory-db/migrations/`.

## Naming Constraints

- Rust crate names: `memory-core`, `memory-db`, `memory-mcp`, `memory-daemon`, `memory-cli`, `memory-api`, `memory-providers`, `memory-tests`.
- Binary name: `agent-memory`.
- SQL enum types use `snake_case`.
- MCP tool names use `snake_case` with `memory.` prefix.

## Codebase Constraints

- `memory-core` must not depend on PostgreSQL, MCP, HTTP, CLI, or specific LLM/embedding providers.
- No business logic in MCP tool handlers — MCP layer validates, authorizes, and delegates to core services.
- No direct SQL in MCP handlers.
- LLM and embedding providers must be behind traits (`EmbeddingProvider`, `ConsolidationProvider`).
- No `#[allow(...)]` attributes without code comment justification.

## Tool Constraints

- **Required:** Rust (latest stable), PostgreSQL >= 15, pgvector >= 0.7.
- **Runtime:** tokio (async), sqlx (database), axum (HTTP), clap (CLI), tracing (logging), serde (serialization).
- **Testing:** cargo test, testcontainers for integration tests.
- **Quality:** cargo fmt, cargo clippy (deny warnings), cargo audit, cargo deny.
- **Documentation:** Context7 for current library documentation during implementation.

## Context Constraints

- Recall token budgets: session-start 1000, task 1200, file 700, architecture 1800, debugging 1500, preference 200.
- NDJSON records must be single-line, no Markdown, no comments.
- Spec files: 150–400 lines recommended.

## Security Constraints

- Secrets must never be written to durable memory.
- Private blocks must be stripped before any persistence, indexing, or embedding.
- MCP error responses must not leak secrets, raw SQL, connection strings, or private data.
- No panic-based control flow in production paths.
- Soft deletion by default; hard deletion only for privacy/compliance with explicit confirmation.
- Audit log must record all memory mutations with actor, action, before/after.
