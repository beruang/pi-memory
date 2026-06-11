# Success Criteria

## Acceptance Criteria

- [ ] Memory is stored as structured observations with kind, confidence, sensitivity, status, and evidence.
- [ ] Every durable memory has at least one evidence reference or explicit user confirmation.
- [ ] PostgreSQL is the canonical store; pgvector is used for semantic recall only.
- [ ] Full-text search is supported via PostgreSQL `tsvector`.
- [ ] Hybrid retrieval combines vector similarity, keyword search, structured filters, confidence, recency, and evidence scoring.
- [ ] MCP tools expose: `memory.recall`, `memory.search`, `memory.get`, `memory.write`, `memory.update`, `memory.mark_obsolete`, `memory.consolidate_session`, `memory.link_file`, `memory.list_conflicts`, `memory.resolve_conflict`, `memory.delete`.
- [ ] Secret content (API keys, tokens, passwords, private keys, JWTs, connection strings) is rejected before persistence.
- [ ] Private blocks (`<private>...</private>`) are excluded from event storage, memory extraction, indexing, embedding, search results, and MCP responses.
- [ ] Conflicted memory is never treated as authoritative without explicit resolution.
- [ ] Superseded memory is preserved as historical context but excluded from default recall.
- [ ] Context injection is bounded by configurable token budget per recall type.
- [ ] The memory core crate has zero dependencies on PostgreSQL, MCP, HTTP, CLI, or specific LLM providers.
- [ ] The system can run as a single `agent-memory` binary.
- [ ] Users can inspect, edit, obsolete, delete, and export memory through CLI and review UI.

## Validation Checks

```bash
# Build
cargo build --workspace --all-features

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format
cargo fmt --all -- --check

# Tests
cargo test --workspace --all-features

# Security audit
cargo audit
cargo deny check

# SQL migrations
cargo sqlx prepare --workspace -- --all-features
```

## Artifact Checks

- [ ] `agent-memory` binary builds and responds to all subcommands.
- [ ] Database migrations apply cleanly against PostgreSQL with pgvector and pgcrypto extensions.
- [ ] MCP server starts and responds to `Initialize` and all tool calls.
- [ ] Integration tests pass with a real PostgreSQL instance.
- [ ] Secret detection tests pass with known secret patterns.

## Quality Checks

- [ ] All crates pass `cargo clippy` with `-D warnings`.
- [ ] All crates pass `cargo fmt --check`.
- [ ] `cargo audit` reports zero vulnerabilities.
- [ ] `cargo deny check` passes with configured policy.
- [ ] Unit tests cover core memory lifecycle, privacy filtering, conflict detection, and ranking.
- [ ] Integration tests cover persistence, search, MCP tools, and migrations.

## Done/Not-Done Boundary

**Done:** The system stores structured, source-backed observations in PostgreSQL. It retrieves relevant memories via hybrid search. It exposes MCP tools for agents. It rejects secrets and respects private blocks. It detects conflicts. It enforces token budgets. It passes all lint, format, test, audit, and deny checks. It builds as a single binary.

**Not done:** The system does not implement raw transcript archiving, does not auto-merge contradictory memories, does not store secrets, and does not put business logic in MCP handlers.
