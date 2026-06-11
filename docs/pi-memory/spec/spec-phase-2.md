# Spec Phase 2: Database Schema, Migrations, and Repositories

## Phase Goal

Implement `memory-db` with PostgreSQL schema, sqlx migrations, repository implementations, full-text search, pgvector vector search, and hybrid retrieval SQL.

## Dependencies

- Requires: Phase 1 (core types and enums from memory-core)
- Produces: `crates/memory-db/`

## Existing Code References

- Pattern to follow: sqlx query macros with compile-time checking, migration files in `migrations/` directory.
- Test pattern: Integration tests with testcontainers or local PostgreSQL.
- Config pattern: Database URL from environment variable or config file.
- Reference: `spec.md` sections 15 (PostgreSQL and pgvector), 16 (Hybrid Retrieval).

## Technical Approach

Use `sqlx` with compile-time checked queries. Define migrations as ordered SQL files. Repository structs hold a `PgPool`. Search functions build parameterized hybrid queries.

**Key design decisions:**
- Use `sqlx::Type` enums already defined in `memory-core` for database enum columns.
- The `search_tsv` column is GENERATED ALWAYS AS STORED — no application-level maintenance.
- Embeddings table uses `vector(N)` where N matches the configured embedding model dimension.
- HNSW index on embeddings for fast approximate nearest neighbor search.
- Hybrid query uses CTEs (vector_matches, text_matches, combined) as defined in spec section 16.2.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/memory-db/Cargo.toml` | sqlx (postgres, runtime-tokio), uuid, time, tracing, memory-core |
| `crates/memory-db/src/lib.rs` | Re-exports, connection pool creation |
| `crates/memory-db/src/postgres.rs` | PgPool setup, run_migrations |
| `crates/memory-db/src/observations_repo.rs` | ObservationsRepository with CRUD + hybrid search |
| `crates/memory-db/src/evidence_repo.rs` | EvidenceRepository |
| `crates/memory-db/src/embeddings_repo.rs` | EmbeddingsRepository with vector search |
| `crates/memory-db/src/search_repo.rs` | Hybrid search query builder |
| `crates/memory-db/src/conflicts_repo.rs` | ConflictsRepository |
| `crates/memory-db/src/migrations.rs` | Migration runner wrapping sqlx::migrate! |
| `crates/memory-db/migrations/0001_initial_schema.sql` | Full schema: extensions, enums, tables, indexes |

### Modified Files

- `Cargo.toml` (workspace root) — add `memory-db` to workspace members.

## Implementation Steps

1. Create `crates/memory-db/Cargo.toml` with sqlx, uuid, time, tracing dependencies.
2. Write `migrations/0001_initial_schema.sql` with all DDL from spec sections 15.1–15.13.
3. Implement `src/postgres.rs` — `create_pool(url: &str) -> PgPool`, `run_migrations(pool: &PgPool)`.
4. Implement `src/observations_repo.rs`:
   - `insert(observation: &Observation) -> Result<Observation>`
   - `get_by_id(id: Uuid) -> Result<Option<Observation>>`
   - `update(observation: &Observation) -> Result<Observation>`
   - `hybrid_search(params: SearchParams) -> Result<Vec<ScoredObservation>>`
   - `list_by_scope(scope: MemoryScope, project_id: Option<Uuid>) -> Result<Vec<Observation>>`
5. Implement `src/evidence_repo.rs` — `insert`, `get_by_observation_id`, `delete_by_observation_id`.
6. Implement `src/embeddings_repo.rs` — `upsert_embedding`, `vector_search`, `delete_by_observation_id`.
7. Implement `src/search_repo.rs` — the hybrid query CTE from spec section 16.2, parameterized by query vector, scope, project_id, text query, limit.
8. Implement `src/conflicts_repo.rs` — `insert_conflict`, `list_open_conflicts`, `resolve_conflict`.
9. Implement `src/migrations.rs` — thin wrapper around `sqlx::migrate!()`.
10. Wire `src/lib.rs` re-exports.

## Data / API / Interface Contract

```rust
pub struct SearchParams {
    pub query_embedding: Vec<f32>,
    pub text_query: String,
    pub scope: MemoryScope,
    pub project_id: Option<Uuid>,
    pub kinds: Option<Vec<MemoryKind>>,
    pub files: Option<Vec<String>>,
    pub limit: u32,
    pub min_confidence: Option<MemoryConfidence>,
}

pub struct ScoredObservation {
    pub observation: Observation,
    pub final_score: f64,
    pub vector_score: f64,
    pub text_score: f64,
}
```

## Error Handling

- All repository methods return `Result<_, MemoryError>`.
- `MemoryError::ObservationNotFound(Uuid)` — get/update on missing ID.
- `MemoryError::Database(sqlx::Error)` — propagated from sqlx.
- Connection errors surfaced at pool creation time.

## Observability

- Logs: `tracing::info` for migration runs, `tracing::debug` for query execution times.
- Metrics: query latency histograms, pool utilization.
- Traces: span per repository method with sqlx query annotation.

## Testing Requirements

### Unit Tests

None — this crate is persistence-only; unit tests would mock the database, which is explicitly discouraged per spec section 3.

### Integration Tests

- Migration round-trip: up then down, verify no leftovers.
- Observation CRUD: insert, read, update, soft delete.
- Evidence CRUD: insert, read by observation, cascade on observation delete.
- Full-text search: insert known text, search with matching and non-matching queries.
- Vector search: insert known embeddings, search with similar and dissimilar vectors.
- Hybrid search: combined text + vector query, verify scoring order.
- Supersession: create pair, verify superseded_by reference.
- Conflict: create conflict pair, verify detection, resolve.

### Regression Tests

- Schema migration idempotency (run twice, no errors).

## Validation Commands

```bash
# Ensure PostgreSQL with pgvector is running
cargo build -p memory-db
cargo test -p memory-db -- --test-threads=1
cargo clippy -p memory-db -- -D warnings
cargo sqlx prepare --workspace
```

## Acceptance Criteria

- [ ] All migrations apply cleanly with `vector`, `pgcrypto`, `pg_trgm` extensions.
- [ ] All repository integration tests pass against real PostgreSQL.
- [ ] Hybrid search returns results ordered by combined score.
- [ ] Full-text search uses `ts_rank_cd` and matches English text.
- [ ] Vector search uses `<=>` cosine distance.
- [ ] HNSW index is created on `observation_embeddings(embedding)`.
- [ ] Cascade deletes work: deleting an observation removes evidence, files, entities, commands, embeddings.
- [ ] `search_tsv` column is auto-generated and updated on insert/update.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Migration conflicts between developers | medium | Single migration file initially; use timestamped migration naming convention |
| HNSW index build time on large datasets | medium | Benchmark with synthetic data after initial implementation |
| sqlx compile-time checking requires running database | low | Document `DATABASE_URL` requirement; use `sqlx prepare` for CI |
