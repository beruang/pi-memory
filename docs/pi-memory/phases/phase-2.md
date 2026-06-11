# Phase 2: Database Schema, Migrations, and Repositories

**Depends on:** Phase 1 (core types and enums)
**Risk:** high — canonical store; schema errors are expensive to fix
**Value:** Persistent storage with full-text search, vector search, and hybrid retrieval.

## Purpose

Implement `memory-db` with PostgreSQL schema, sqlx-compiled queries, migrations, and repository implementations. This crate depends on `memory-core` for types and enums.

## Deliverables

- SQL migrations for all tables, enums, and indexes from spec sections 15.1–15.13.
- Enum types: memory_scope, memory_kind, memory_confidence, memory_sensitivity, memory_status.
- Tables: observations (with generated tsvector column), evidence, observation_files, observation_entities, observation_commands, observation_supersessions, observation_conflicts, observation_embeddings, session_events, memory_audit_log.
- Indexes: scope+project, user, org, kind, created_at, status, GIN on search_tsv, GIN on metadata, HNSW on embedding.
- PostgreSQL extensions: vector, pgcrypto, pg_trgm.
- Repository traits and implementations for: observations (CRUD, search, hybrid query), evidence, files, entities, commands, embeddings, conflicts, audit log, session events.
- Hybrid retrieval SQL function from spec section 16.2.
- Full-text search via `ts_rank_cd` and `plainto_tsquery`.
- Vector search via pgvector `<=>` cosine distance operator.
- Connection pool management with sqlx.
- Migration runner.

## Files

| File | Purpose |
|------|---------|
| `crates/memory-db/Cargo.toml` | sqlx, uuid, chrono, tokio dependencies |
| `crates/memory-db/src/lib.rs` | Public re-exports |
| `crates/memory-db/src/postgres.rs` | Connection pool, config |
| `crates/memory-db/src/observations_repo.rs` | Observation CRUD, search, hybrid query |
| `crates/memory-db/src/evidence_repo.rs` | Evidence CRUD |
| `crates/memory-db/src/embeddings_repo.rs` | Embedding storage and vector search |
| `crates/memory-db/src/search_repo.rs` | Full-text and hybrid search queries |
| `crates/memory-db/src/conflicts_repo.rs` | Conflict storage and query |
| `crates/memory-db/src/migrations.rs` | Migration runner |
| `crates/memory-db/migrations/*.sql` | SQL migration files |

## Testing

- Migration tests: up and down against real PostgreSQL with pgvector.
- Repository integration tests: CRUD for each entity.
- Full-text search tests with known queries and expected results.
- Vector search tests with synthetic embeddings.
- Hybrid search tests combining filters, text, and vector.
- Supersession and conflict link tests.
