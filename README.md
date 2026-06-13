# PI Memory

**Persistent, structured, source-backed memory for AI agents.**

[![CI](https://github.com/beruang/pi-memory/actions/workflows/ci.yml/badge.svg)](https://github.com/beruang/pi-memory/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/badge/crate-unpublished-gray)](https://crates.io)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange)](https://www.rust-lang.org)
[![PostgreSQL](https://img.shields.io/badge/postgres-16%2B-336791)](https://www.postgresql.org)

PI Memory gives AI agents durable recall across sessions — facts, decisions, constraints, preferences, bugs, and failed attempts — without storing raw conversation transcripts. Memory is selective, grounded, safe, and expires naturally.

---

## Table of Contents

- [Features](#features)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [MCP Tools](#mcp-tools)
- [CLI Reference](#cli-reference)
- [Memory Lifecycle](#memory-lifecycle)
- [Privacy & Safety](#privacy--safety)
- [Search & Recall](#search--recall)
- [Development](#development)
- [License](#license)

---

## Features

- **11 MCP tools** — comprehensive memory operations over stdio
- **Hybrid retrieval** — pgvector HNSW (semantic) + PostgreSQL tsvector GIN (keyword) + structured filters
- **Multi-factorial ranking** — semantic similarity, keyword relevance, confidence, recency decay, evidence count, file/entity match, status penalties
- **Conflict detection** — automatically detects contradictory observations on shared entities
- **Privacy pipeline** — `<private>` block stripping, secret regex scanning, sensitivity classification
- **Lifecycle FSM** — active → confirmed → obsolete → superseded → deleted with valid transition enforcement
- **Full audit trail** — every mutation logged with actor identity and before/after state
- **Supersession tracking** — older observations marked superseded when replaced
- **Background workers** — session consolidation, embedding generation, event cleanup
- **Pluggable providers** — mock, OpenAI, Ollama, Context7 for embeddings and consolidation

---

## Architecture

```
AI agent / coding assistant
  │
  ▼
MCP tools (stdio transport)
  │
  ▼
Memory MCP server  ──►  HTTP REST API
  │                         │
  ▼                         ▼
Memory core (domain logic, FSM, privacy, conflicts)
  │
  ▼
PostgreSQL + pgvector (hybrid search, HNSW indexes)
```

The workspace is organized into 9 crates:

| Crate | Purpose |
|-------|---------|
| `memory-core` | Domain model, lifecycle FSM, conflict detection, privacy scanning, ranking |
| `memory-db` | PostgreSQL schema, migrations, repositories (sqlx) |
| `memory-mcp` | MCP server with 11 tools over stdio transport |
| `memory-providers` | Embedding & consolidation traits + mock/OpenAI/Ollama/Context7 |
| `memory-daemon` | Background workers: consolidation, embedding, cleanup, conflict detection |
| `memory-cli` | CLI binary (`agent-memory`) |
| `memory-api` | HTTP REST API (Axum) |
| `memory-config` | TOML + env config loading via figment |
| `memory-tests` | Integration tests |

---

## Quick Start

### Prerequisites

- **Rust 1.80+** (MSRV)
- **PostgreSQL 16+** with **pgvector** extension
- `cargo`, `git`

### 1. Set up the database

```bash
# Create database
createdb agent_memory

# Enable pgvector
psql -d agent_memory -c "CREATE EXTENSION IF NOT EXISTS vector;"
```

### 2. Build and migrate

```bash
git clone https://github.com/beruang/pi-memory.git
cd pi-memory

cargo build --release
cargo run --release -p memory-cli -- migrate
```

### 3. Run the MCP server

```bash
cargo run --release -p memory-cli -- mcp
```

The server reads one JSON-RPC request per line from stdin and writes one JSON-RPC response per line to stdout. Connect your AI agent to use it.

### 4. (Optional) Run the HTTP API

```bash
cargo run --release -p memory-cli -- serve --port 8080
```

---

## Configuration

Create `agent-memory.toml` in the project root or working directory:

```toml
database_url = "postgres://localhost:5432/agent_memory"
default_scope = "project"
event_retention_days = 30
max_recall_tokens = 1200
enable_private_blocks = true
enable_secret_scanner = true

[embedding]
kind = "mock"       # "mock" | "openai" | "ollama"
model = "mock"
dimensions = 1536

[api]
host = "0.0.0.0"
port = 8080
```

All config values can be overridden via environment variables with the `AGENT_MEMORY__` prefix:

```bash
export AGENT_MEMORY__DATABASE_URL="postgres://localhost:5432/agent_memory"
export AGENT_MEMORY__EMBEDDING__KIND="openai"
export AGENT_MEMORY__EMBEDDING__API_KEY_ENV="OPENAI_API_KEY"
```

---

## MCP Tools

| Tool | Description |
|------|-------------|
| `memory.write` | Store a new observation with privacy scan, conflict detection, embedding generation |
| `memory.get` | Retrieve a single observation by ID with evidence, files, entities |
| `memory.update` | Update observation content, validate lifecycle transition, regenerate embedding |
| `memory.delete` | Soft-delete or hard-delete an observation |
| `memory.search` | Hybrid search with vector + keyword + structured filters |
| `memory.recall` | Task-aware recall with token budget and confidence filters |
| `memory.consolidate_session` | Process session events into structured observations |
| `memory.session_start` | Load relevant context at session start within a token budget |
| `memory.link_file` | Attach a file path to an existing observation |
| `memory.list_conflicts` | List unresolved conflicts for a project |
| `memory.resolve_conflict` | Resolve a conflict (left_wins / right_wins / merge) |
| `memory.mark_obsolete` | Mark observation as obsolete with lifecycle enforcement |

### Tool: `memory.write`

```json
{
  "scope": "project",
  "kind": "decision",
  "summary": "Use PostgreSQL for primary storage",
  "confidence": "high",
  "sensitivity": "internal",
  "project_id": "550e8400-e29b-41d4-a716-446655440000",
  "entities": ["database", "postgresql"],
  "evidence": [
    {"source_type": "message", "source_id": "msg_123", "excerpt": "Decision made during architecture review"}
  ]
}
```

### Tool: `memory.search`

```json
{
  "query": "auth middleware design",
  "scope": "project",
  "project_id": "550e8400-e29b-41d4-a716-446655440000",
  "limit": 10,
  "kinds": ["decision", "constraint"],
  "min_confidence": "medium"
}
```

### Tool: `memory.recall`

```json
{
  "task": "debug the failing authentication flow",
  "scope": "project",
  "project_id": "550e8400-e29b-41d4-a716-446655440000",
  "token_budget": 500,
  "min_confidence": "low"
}
```

---

## CLI Reference

```bash
# MCP server (stdio transport)
cargo run --release -p memory-cli -- mcp

# HTTP API + background workers
cargo run --release -p memory-cli -- serve --port 8080

# Run database migrations
cargo run --release -p memory-cli -- migrate

# Search memories
cargo run --release -p memory-cli -- search "auth middleware" --scope project

# Recall for a task
cargo run --release -p memory-cli -- recall --task "debug failed auth tests" --scope project

# Consolidate a session
cargo run --release -p memory-cli -- consolidate --session <session-id>

# Inspect a single observation
cargo run --release -p memory-cli -- inspect <observation-id>

# Start review UI
cargo run --release -p memory-cli -- ui --port 3000
```

---

## Memory Lifecycle

```
active ──confirm──► confirmed ──obsolete──► obsolete
  │                                   ▲
  │                                   │
  └──supersede──► superseded ──────────┘
  │
  └──delete──► deleted
```

| Status | Description |
|--------|-------------|
| **active** | Newly written, available in search and recall |
| **confirmed** | Verified by user, boosted in ranking |
| **obsolete** | Outdated, penalized in ranking |
| **superseded** | Replaced by newer observation, hidden from default search |
| **conflicted** | Contradicts another active observation, pending resolution |
| **deleted** | Soft-deleted, removed from search, audit-logged |

Transitions are enforced by a finite state machine — invalid transitions return a descriptive error.

---

## Privacy & Safety

PI Memory actively prevents sensitive content from being persisted:

- **`<private>` block stripping** — content between `<private>` and `</private>` tags is removed before storage
- **Secret scanning** — regex-based detection of API keys, tokens, database credentials, private keys
- **Sensitivity classification** — `public` / `internal` / `private` / `secret` with automatic upgrade when private blocks are stripped
- **Secrets excluded from search** — observations classified as `secret` are filtered out of hybrid search results
- **Full audit trail** — all writes and redactions logged without storing the private content itself

---

## Search & Recall

### Hybrid Search (`memory.search`)

Combines three retrieval strategies with multi-factorial ranking:

1. **Semantic** — pgvector `<=>` cosine distance on 1536-dimensional embeddings (HNSW index)
2. **Keyword** — PostgreSQL `tsvector` full-text search with `ts_rank_cd` (GIN index)
3. **Structured filters** — scope, project, kind, file path, entity name, confidence threshold

**Ranking formula:**

```
final_score = vector_score × 0.45
            + text_score    × 0.30
            + confidence    × 0.10
            + recency       × 0.05
            + evidence      × 0.05
            + file/entity   × 0.05
            - status_penalty
```

### Task-Aware Recall (`memory.recall`)

Queries across multiple knowledge domains — architecture, decisions, constraints, preferences, policies, dependencies, procedures — and collects results within a configurable token budget.

### Session Start (`memory.session_start`)

Pre-populates agent context at session initialization by running domain-targeted queries, deduplicating results, and returning a token-budgeted context pack.

---

## Development

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Test (requires DATABASE_URL pointing to a running PostgreSQL instance)
DATABASE_URL="postgres://localhost:5432/agent_memory" cargo test --workspace --all-features

# Full quality gate
cargo fmt --all && \
  cargo clippy --workspace --all-targets --all-features -- -D warnings && \
  DATABASE_URL="postgres://localhost:5432/agent_memory" cargo test --workspace --all-features

# Security audit
cargo audit

# License/Dependency check
cargo deny check
```

### Project Structure

```
├── crates/
│   ├── memory-core/         # Domain model, FSM, business logic
│   ├── memory-db/           # PostgreSQL layer (sqlx)
│   ├── memory-mcp/          # MCP protocol server
│   ├── memory-providers/    # Embedding & consolidation providers
│   ├── memory-daemon/       # Background worker processes
│   ├── memory-cli/          # CLI binary entrypoint
│   ├── memory-api/          # HTTP REST API
│   ├── memory-config/       # Configuration loading
│   └── memory-tests/        # Integration tests
├── Cargo.toml
├── deny.toml                # cargo-deny configuration
└── clippy.toml              # Clippy lint configuration
```

### CI Pipeline

The included GitHub Actions workflow runs:

- Format check (`cargo fmt --check`)
- Lint (`cargo clippy` with deny warnings)
- Test (all crates with all features)
- Security audit (`cargo audit`)
- License/dependency check (`cargo deny`)
- MSRV validation
- Caching for faster subsequent runs

---

## License

MIT License. See [LICENSE](LICENSE).
