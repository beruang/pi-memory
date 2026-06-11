# Spec Phase 6: Integration Tests, CI, and Quality Gates

## Phase Goal

Implement `memory-tests` integration test suite, CI pipeline configuration, `deny.toml`, and ensure all quality gates pass.

## Dependencies

- Requires: Phases 1–5 (all crates implemented)
- Produces: `crates/memory-tests/`, `.github/workflows/ci.yml`, `deny.toml`

## Existing Code References

- Pattern to follow: Integration tests in `tests/` directory using testcontainers or external PostgreSQL.
- Test pattern: Setup test database, run migrations, seed data, execute test, assert, teardown.
- CI pattern: GitHub Actions with matrix builds, PostgreSQL service container.
- Reference: `spec.md` sections 26 (Linting/Formatting/Checks), 27 (Development Quality Rules), 38 (Acceptance Criteria).

## Technical Approach

Integration tests run against a real PostgreSQL instance with pgvector. Use testcontainers-rs for automated database provisioning in tests, or a pre-configured `DATABASE_URL` for local development. CI uses a GitHub Actions service container for PostgreSQL.

**Key design decisions:**
- Integration tests are in a separate crate to avoid circular dev-dependencies.
- Test database is created per test run, migrated, and torn down.
- CI must run fmt, clippy, test, audit, and deny as separate steps.
- `deny.toml` must be configured before `cargo deny check` can pass.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/memory-tests/Cargo.toml` | Dev-dependencies on all workspace crates, testcontainers, tokio |
| `crates/memory-tests/tests/integration_memory.rs` | End-to-end memory lifecycle tests |
| `crates/memory-tests/tests/integration_search.rs` | Hybrid search accuracy and performance tests |
| `crates/memory-tests/tests/integration_mcp.rs` | MCP server integration tests |
| `.github/workflows/ci.yml` | CI pipeline definition |
| `deny.toml` | cargo-deny configuration |

### Modified Files

- `Cargo.toml` (workspace root) — add `memory-tests` to workspace members.

## Implementation Steps

1. Create `crates/memory-tests/Cargo.toml` with dev-dependencies on memory-core, memory-db, memory-mcp, memory-daemon, memory-providers. Add testcontainers, tokio.
2. Implement `tests/integration_memory.rs`:
   - Full lifecycle: write observation → retrieve → update → mark obsolete → verify status.
   - Privacy: write with secret → rejected. Write with private block → block stripped.
   - Conflict: write conflicting observations → conflict detected.
   - Supersession: create pair, supersede old with new, verify links.
   - Audit trail: verify audit log records all mutations.
3. Implement `tests/integration_search.rs`:
   - Seed diverse observations (different kinds, files, entities).
   - Hybrid search with text query → verify relevant results rank higher.
   - Vector search with embedding → verify semantic matches.
   - File-filtered search → verify only linked-file observations returned.
   - Kind-filtered search → verify only requested kinds returned.
4. Implement `tests/integration_mcp.rs`:
   - Start MCP server on test transport.
   - Call each of the 11 tools with valid inputs.
   - Verify error handling for invalid inputs.
   - Verify token budget enforcement.
   - Verify authorization boundaries.
5. Create `.github/workflows/ci.yml`:
   ```yaml
   name: CI
   on: [push, pull_request]
   jobs:
     check:
       runs-on: ubuntu-latest
       services:
         postgres:
           image: pgvector/pgvector:pg16
           env:
             POSTGRES_PASSWORD: postgres
           ports:
             - 5432:5432
       steps:
         - uses: actions/checkout@v4
         - uses: actions-rs/toolchain@v1
         - run: cargo fmt --all -- --check
         - run: cargo clippy --workspace --all-targets --all-features -- -D warnings
         - run: cargo test --workspace --all-features
         - run: cargo audit
         - run: cargo deny check
   ```
6. Create `deny.toml`:
   - Ban licenses: none initially (warn on unknown).
   - Deny vulnerabilities: critical and high.
   - Deny unmaintained crates.
   - Allow-list for common licenses: MIT, Apache-2.0, BSD, ISC, MPL-2.0, Unicode-DFS-2016.
   - Warn on multiple versions of key crates (tokio, sqlx, serde).

## Testing Requirements

### Integration Tests

All tests in `crates/memory-tests/tests/`:
- Memory lifecycle end-to-end.
- Hybrid search with real PostgreSQL + pgvector.
- MCP tool calls against running server.

### Regression Tests

- Secret redaction regression: known secret patterns must be rejected.
- Migration up/down round-trip without data loss.
- CI green on clean checkout.

### Performance Smoke Tests

- Recall latency < 500ms with 10K observations.
- Write + index latency < 200ms.

## Validation Commands

```bash
# Full CI simulation
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
cargo deny check

# NDJSON validation
python3 .pi/skills/brainstorm/scripts/validate_ndjson.py .agent/contracts/pi-memory/*.ndjson
python3 .pi/skills/brainstorm/scripts/validate_brainstorm_artifacts.py pi-memory
```

## Acceptance Criteria

- [ ] All integration tests pass against real PostgreSQL with pgvector.
- [ ] CI pipeline runs fmt, clippy, test, audit, deny.
- [ ] CI provisions PostgreSQL with pgvector (service container or testcontainers).
- [ ] `cargo audit` reports zero vulnerabilities.
- [ ] `cargo deny check` passes.
- [ ] `cargo sqlx prepare --workspace` succeeds (offline query checking).
- [ ] Memory lifecycle test covers write → recall → verify → update → obsolete.
- [ ] Privacy test verifies secrets rejected and private blocks stripped.
- [ ] MCP test covers all 11 tools.
- [ ] All NDJSON artifacts validate.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| CI PostgreSQL service container incompatible with pgvector version | medium | Use `pgvector/pgvector:pg16` Docker image explicitly |
| Testcontainers not available in CI environment | low | Use CI service container as fallback; feature-flag testcontainers |
| `cargo audit` finds vulnerabilities in dependencies | medium | Update dependencies; if no fix available, add temporary allow with comment |
| Integration tests slow down CI | low | Parallelize tests where possible; use test database per test for isolation |
