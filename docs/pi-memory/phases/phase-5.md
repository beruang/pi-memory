# Phase 5: Daemon, CLI, and HTTP API

**Depends on:** Phase 1 (core types), Phase 2 (database), Phase 3 (providers)
**Risk:** medium — multiple binaries and interfaces
**Value:** Operational system: background workers, human-facing CLI, and HTTP API for review UI.

## Purpose

Implement `memory-daemon`, `memory-cli`, and `memory-api` to complete the operational surface of the system.

## Deliverables

### memory-daemon
- Consolidation queue and worker: process session events into observations.
- Embedding worker: generate embeddings for new/updated observations.
- Cleanup job: expire old session events per retention config.
- Conflict detection job: periodic scan for new conflicts.
- Scheduled maintenance: index reindexing, stale memory detection.
- Job queue with retry and dead-letter handling.

### memory-cli
- `agent-memory serve` — run daemon, API, and optional UI.
- `agent-memory mcp` — run MCP server over stdio.
- `agent-memory migrate` — run PostgreSQL migrations.
- `agent-memory search "query"` — human-facing hybrid search.
- `agent-memory recall --task "..."` — task-aware recall.
- `agent-memory consolidate --session <id>` — manual consolidation.
- `agent-memory inspect <id>` — display observation with provenance.
- `agent-memory ui` — start review UI.
- CLI argument parsing with clap derive macros.

### memory-api
- Axum HTTP server with routes for memory CRUD, search, recall.
- JSON request/response bodies.
- Localhost binding by default.
- Routes: GET /observations, GET /observations/:id, POST /observations, PUT /observations/:id, DELETE /observations/:id, GET /search, GET /conflicts, POST /conflicts/:id/resolve.
- Optional review UI serving static assets.

## Files

| File | Purpose |
|------|---------|
| `crates/memory-daemon/Cargo.toml` | tokio, sqlx, tracing dependencies |
| `crates/memory-daemon/src/lib.rs` | Public re-exports |
| `crates/memory-daemon/src/worker.rs` | Worker loop and job processing |
| `crates/memory-daemon/src/queue.rs` | Job queue with retry |
| `crates/memory-daemon/src/scheduler.rs` | Cron-like scheduler for periodic jobs |
| `crates/memory-cli/Cargo.toml` | clap, tokio dependencies |
| `crates/memory-cli/src/main.rs` | Binary entry point |
| `crates/memory-cli/src/commands.rs` | Subcommand implementations |
| `crates/memory-api/Cargo.toml` | axum, serde, tower dependencies |
| `crates/memory-api/src/lib.rs` | Public re-exports |
| `crates/memory-api/src/routes.rs` | Route definitions |
| `crates/memory-api/src/handlers.rs` | Request handlers delegating to core |

## Testing

- Daemon worker unit tests with mock providers.
- CLI argument parsing tests.
- API handler integration tests with test database.
- End-to-end test: `agent-memory serve` → API call → observation persisted.
