# Spec Phase 5: Daemon, CLI, and HTTP API

## Phase Goal

Implement `memory-daemon`, `memory-cli`, and `memory-api` to provide background workers, human-facing CLI, and local HTTP API for the review UI.

## Dependencies

- Requires: Phase 1 (core types), Phase 2 (database repositories), Phase 3 (providers)
- Produces: `crates/memory-daemon/`, `crates/memory-cli/`, `crates/memory-api/`

## Existing Code References

- Pattern to follow: tokio-based worker loop with channels, clap derive for CLI, axum for HTTP.
- Test pattern: In-process integration tests with test database and mock providers.
- Config pattern: Figment or config crate for layered config (file + env).
- Reference: `spec.md` sections 7 (CLI commands), 22 (Consolidation), 30 (Review UI), 32 (Configuration).

## Technical Approach

Three crates, each with a distinct purpose:

**memory-daemon:** tokio-based background workers communicating via channels. Consolidation worker pulls from queue, calls ConsolidationProvider, writes observations via memory-db. Embedding worker generates embeddings for observations missing them. Scheduler triggers periodic cleanup, conflict detection, and maintenance.

**memory-cli:** clap derive-based CLI with 8 subcommands. Each subcommand instantiates the needed services and executes the operation.

**memory-api:** axum HTTP server with typed routes. Serves CRUD endpoints for the review UI. Binds to localhost by default.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/memory-daemon/Cargo.toml` | tokio, sqlx, tracing, memory-core, memory-db, memory-providers |
| `crates/memory-daemon/src/lib.rs` | Re-exports, daemon builder |
| `crates/memory-daemon/src/worker.rs` | Consolidation worker, embedding worker |
| `crates/memory-daemon/src/queue.rs` | Job queue with Postgres-backed persistence |
| `crates/memory-daemon/src/scheduler.rs` | Periodic job scheduler |
| `crates/memory-cli/Cargo.toml` | clap, tokio, tracing-subscriber, config, memory-core, memory-db |
| `crates/memory-cli/src/main.rs` | Binary entry point, CLI parsing |
| `crates/memory-cli/src/commands.rs` | Subcommand implementations |
| `crates/memory-api/Cargo.toml` | axum, tower, tower-http, serde, serde_json, memory-core, memory-db |
| `crates/memory-api/src/lib.rs` | Re-exports, app builder |
| `crates/memory-api/src/routes.rs` | Route definitions |
| `crates/memory-api/src/handlers.rs` | Request handlers |

### Modified Files

- `Cargo.toml` (workspace root) — add `memory-daemon`, `memory-cli`, `memory-api` to workspace members.

## Implementation Steps

### memory-daemon
1. Implement `src/queue.rs` — `JobQueue` with `enqueue`, `dequeue`, `ack`, `nack` backed by Postgres.
2. Implement `src/worker.rs`:
   - `ConsolidationWorker` — dequeues consolidation jobs, calls ConsolidationProvider, writes observations, marks job done.
   - `EmbeddingWorker` — finds observations without embeddings, calls EmbeddingProvider, stores results.
3. Implement `src/scheduler.rs` — `Scheduler` with cron-like triggers for cleanup, conflict scan, maintenance.

### memory-cli
4. Implement `src/main.rs` — clap app with 8 subcommands: serve, mcp, migrate, search, recall, consolidate, inspect, ui.
5. Implement `src/commands.rs`:
   - `serve` — starts daemon workers + HTTP API + optional UI.
   - `mcp` — starts MCP server over stdio.
   - `migrate` — runs database migrations.
   - `search` — calls hybrid search and prints results.
   - `recall` — calls task-aware recall and prints results.
   - `consolidate` — triggers session consolidation.
   - `inspect` — fetches observation by ID and prints with evidence.
   - `ui` — starts the review UI on a local port.

### memory-api
6. Implement `src/routes.rs` — axum router with routes: GET /health, GET /observations, POST /observations, GET /observations/:id, PUT /observations/:id, DELETE /observations/:id, GET /search, GET /conflicts, POST /conflicts/:id/resolve.
7. Implement `src/handlers.rs` — typed handlers extracting params, calling memory-db repositories, returning JSON.

## Data / API / Interface Contract

### CLI
```bash
agent-memory serve [--port <port>] [--config <file>]
agent-memory mcp
agent-memory migrate [--database-url <url>]
agent-memory search <query> [--scope <scope>] [--project-id <id>] [--limit <n>]
agent-memory recall --task <task> [--scope <scope>] [--project-id <id>] [--files <paths>]
agent-memory consolidate --session <session-id> [--project-id <id>]
agent-memory inspect <observation-id>
agent-memory ui [--port <port>]
```

### HTTP API
```
GET    /health
GET    /observations?scope=&project_id=&kind=&status=&limit=&offset=
POST   /observations          { observation fields }
GET    /observations/:id
PUT    /observations/:id      { partial update fields }
DELETE /observations/:id      { reason }
GET    /search?q=&scope=&project_id=&kinds=&limit=
GET    /conflicts?project_id=&status=
POST   /conflicts/:id/resolve { resolution, reason }
```

## Error Handling

- CLI: print errors to stderr, exit non-zero on failure.
- HTTP API: return JSON error responses with status codes (404, 400, 422, 500).
- Daemon: log errors, retry with backoff, dead-letter after max retries.

## Observability

- Logs: `tracing::info` for CLI command start/end, daemon job lifecycle, HTTP request/response.
- Metrics: daemon queue depth, job latency, API request latency, error rates.
- Traces: spans for each CLI command, HTTP request, daemon job.

## Testing Requirements

### Unit Tests

- CLI argument parsing: verify each subcommand parses correctly.
- API handler input validation: invalid JSON, missing fields, invalid UUIDs.
- Daemon queue: enqueue/dequeue/ack/nack cycle.

### Integration Tests

- CLI `search` against test database with known data.
- CLI `inspect` against known observation ID.
- API CRUD round-trip: POST → GET → PUT → GET → DELETE → GET (404).
- API search endpoint with query parameters.
- Daemon worker processes consolidation job end-to-end with mock provider.

### Regression Tests

- CLI help output includes all subcommands.
- API returns CORS headers for review UI (if local development).
- Daemon handles database connection loss gracefully (retry, not panic).

## Validation Commands

```bash
cargo build -p memory-daemon -p memory-cli -p memory-api
cargo test -p memory-daemon -p memory-cli -p memory-api
cargo clippy -p memory-daemon -p memory-cli -p memory-api -- -D warnings
```

## Acceptance Criteria

- [ ] `agent-memory serve` starts daemon + API + workers.
- [ ] `agent-memory mcp` starts MCP server on stdio.
- [ ] `agent-memory migrate` applies migrations.
- [ ] `agent-memory search "query"` returns ranked results.
- [ ] `agent-memory recall --task "..."` returns task-relevant memories.
- [ ] `agent-memory inspect <id>` shows observation with evidence.
- [ ] `agent-memory ui` serves review UI on localhost.
- [ ] HTTP API CRUD endpoints work correctly.
- [ ] Consolidation worker processes jobs end-to-end.
- [ ] Embedding worker generates embeddings for new observations.
- [ ] Scheduler runs cleanup and conflict detection on schedule.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Daemon worker crashes lose jobs | medium | Persist job queue in Postgres; ack after successful processing |
| CLI startup latency from database connection | low | Lazy connection; only connect when command needs database |
| API exposes private content | high | Apply same sensitivity filters as MCP; never return secret-classified content |
