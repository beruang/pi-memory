# PI Memory

**Persistent, structured, source-backed memory for AI agents.**

PI Memory preserves useful continuity between sessions — facts, decisions, constraints, preferences, bugs, and failed attempts — without storing raw transcripts. Memory is selective, grounded, safe, and expires naturally.

## Architecture

```
AI agent / coding assistant
  │
  ▼
MCP tools (stdio)
  │
  ▼
Memory MCP server (Rust)
  │
  ▼
Memory core (traits + business logic)
  │
  ▼
PostgreSQL + pgvector (hybrid search)
```

PI Memory is built as a Rust workspace with 9 crates:

| Crate | Purpose |
|-------|---------|
| `memory-core` | Domain model, lifecycle FSM, conflict detection, privacy scanning, ranking |
| `memory-db` | PostgreSQL schema, migrations, repositories |
| `memory-mcp` | MCP server with 11 tools over stdio transport |
| `memory-providers` | Embedding and consolidation provider traits + mock/OpenAI/Ollama/Context7 |
| `memory-daemon` | Background workers: consolidation, embedding, cleanup, conflict detection |
| `memory-cli` | CLI binary (`agent-memory`) |
| `memory-api` | HTTP REST API |
| `memory-config` | TOML + env config loading via figment |
| `memory-tests` | Integration tests |

## Features

- **11 MCP tools**: `memory.write`, `memory.get`, `memory.update`, `memory.delete`, `memory.search`, `memory.recall`, `memory.consolidate_session`, `memory.link_file`, `memory.resolve_conflict`, `memory.mark_obsolete`, `memory.inspect`
- **Hybrid retrieval**: pgvector HNSW (semantic) + PostgreSQL tsvector GIN (keyword) + structured filters
- **Ranking**: semantic score, keyword score, confidence, recency decay, evidence count, file/entity match, status penalties
- **Conflict detection**: same-entity contradictory summaries produce `observation_conflicts` records
- **Privacy pipeline**: `<private>` block stripping, secret regex scanning, sensitivity classification
- **Lifecycle FSM**: active → confirmed → obsolete → superseded → deleted with valid transition enforcement
- **Audit log**: every write/update/delete operation recorded with actor, before/after state
- **Supersession**: older observations marked superseded when replaced by newer ones
- **Background workers**: session consolidation, embedding generation, event cleanup, conflict detection
- **Config**: TOML file + `AGENT_MEMORY__*` env var overrides via figment

## Prerequisites

- **Rust 1.80+** (MSRV)
- **PostgreSQL 16+** with **pgvector** extension
- `cargo`, `git`

## Quick Start

### 1. Set up the database

```bash
# Create database
createdb agent_memory

# Enable pgvector
psql -d agent_memory -c "CREATE EXTENSION IF NOT EXISTS vector;"
```

### 2. Configure

Create `agent-memory.toml` in the project root or current working directory:

```toml
database_url = "postgres://localhost:5432/agent_memory"
default_scope = "project"
event_retention_days = 30
max_recall_tokens = 1200
enable_private_blocks = true
enable_secret_scanner = true

[embedding]
kind = "mock"       # or "openai", "ollama"
model = "mock"
dimensions = 1536

[api]
host = "0.0.0.0"
port = 8080
```

Or use environment variables:

```bash
export AGENT_MEMORY__DATABASE_URL="postgres://localhost:5432/agent_memory"
export AGENT_MEMORY__EMBEDDING__KIND="openai"
export AGENT_MEMORY__EMBEDDING__API_KEY_ENV="OPENAI_API_KEY"
```

### 3. Build and migrate

```bash
cargo build --release
cargo run -p memory-cli -- migrate
```

### 4. Run the MCP server

```bash
cargo run -p memory-cli -- mcp
```

The MCP server runs over stdio. Connect your AI agent (Claude Code, etc.) to use it.

### 5. Run the HTTP API (optional)

```bash
cargo run -p memory-cli -- serve --port 8080
```

### 6. Run the daemon with workers (optional)

```bash
cargo run -p memory-cli -- serve --port 8080
# Daemon starts API + scheduler + workers together
```

## CLI Commands

```bash
# MCP server (stdio transport)
cargo run -p memory-cli -- mcp

# HTTP API + background workers
cargo run -p memory-cli -- serve --port 8080

# Run database migrations
cargo run -p memory-cli -- migrate

# Search memories
cargo run -p memory-cli -- search "auth middleware" --scope project

# Recall for a task
cargo run -p memory-cli -- recall --task "debug failed auth tests"

# Consolidate a session
cargo run -p memory-cli -- consolidate --session <session-id>

# Inspect a single observation
cargo run -p memory-cli -- inspect <observation-id>

# Start review UI
cargo run -p memory-cli -- ui --port 3000
```

## MCP Tools

| Tool | Description |
|-----|-------------|
| `memory.write` | Store a new observation with privacy scan, conflict detection, embedding generation |
| `memory.get` | Retrieve a single observation by ID with evidence, files, entities |
| `memory.update` | Update observation content, validate lifecycle transition, regenerate embedding |
| `memory.delete` | Soft-delete observation (lifecycle enforcement) |
| `memory.search` | Hybrid search with vector + keyword + structured filters |
| `memory.recall` | Task-aware recall with confidence and kind filters |
| `memory.consolidate_session` | Enqueue session consolidation job |
| `memory.link_file` | Attach a file path to an existing observation |
| `memory.resolve_conflict` | Resolve a conflict (left_wins / right_wins / merge) |
| `memory.mark_obsolete` | Mark observation as obsolete (lifecycle enforcement) |
| `memory.inspect` | Retrieve observation with full provenance chain |

## Development

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Test
cargo test --workspace --all-features

# Full check (fmt + clippy + test + audit + deny)
cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo audit && cargo deny check
```

## Memory Lifecycle

```
active ──confirm──► confirmed ──obsolete──► obsolete
  │                                   ▲
  │                                   │
  └──supersede──► superseded ──────────┘
  │
  └──delete──► deleted
```

- **active**: newly written, available in search
- **confirmed**: verified by user, boosted in recall
- **obsolete**: outdated, penalized in ranking, hidden from default search
- **superseded**: replaced by a newer observation, hidden from default search
- **deleted**: soft-deleted, removed from search, audit-logged

## Privacy

PI Memory actively prevents sensitive content from being stored:

- **`<private>` blocks**: stripped from summary before storage
- **Secret scanning**: regex detection of API keys, tokens, credentials
- **Sensitivity classification**: `internal` / `private` / `secret` — secrets excluded from search by default
- **Audit log**: all writes/redactions logged without storing the private content itself

## Conflict Detection

When `memory.write` detects that a new observation contradicts an existing one on the same entities and kind, it:

1. Marks the new observation as `conflicted`
2. Inserts an `observation_conflicts` record
3. The `memory.resolve_conflict` tool lets agents or users pick a winner or merge

## Contributing

Contributions are welcome. Please ensure:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## License

MIT License. See [LICENSE](LICENSE).
