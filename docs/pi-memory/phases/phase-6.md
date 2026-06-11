# Phase 6: Integration Tests, CI, and Quality Gates

**Depends on:** Phases 1–5 (all crates)
**Risk:** medium — quality infrastructure
**Value:** Confidence that the system works end-to-end and maintains quality standards.

## Purpose

Implement `memory-tests`, CI configuration, deny.toml, and ensure all quality gates pass.

## Deliverables

- Integration tests: end-to-end memory lifecycle (write → recall → verify → update → obsolete).
- Integration tests: MCP tool calls against a running server.
- Integration tests: hybrid search accuracy with known data.
- Regression tests: secret redaction for known patterns.
- Regression tests: conflict detection for known conflict pairs.
- CI configuration (GitHub Actions or similar): fmt, clippy, test, audit, deny.
- CI PostgreSQL+pgvector provisioning (testcontainers or service container).
- `deny.toml` with duplicate, vulnerability, license, and unmaintained crate policies.
- `cargo sqlx prepare` workflow for offline query checking.
- Migration tests: up/down cycle without data loss.
- Performance smoke tests: recall latency under load.

## Files

| File | Purpose |
|------|---------|
| `crates/memory-tests/Cargo.toml` | Dev-dependencies on all crates |
| `crates/memory-tests/tests/integration_memory.rs` | Memory lifecycle tests |
| `crates/memory-tests/tests/integration_search.rs` | Hybrid search tests |
| `crates/memory-tests/tests/integration_mcp.rs` | MCP tool tests |
| `.github/workflows/ci.yml` | CI pipeline |
| `deny.toml` | cargo-deny configuration |
| `.cargo/config.toml` | SQLx offline mode config (if needed) |

## Testing

- CI must pass all checks: fmt, clippy, test, audit, deny.
- Integration tests must run against real PostgreSQL with pgvector.
- Migration tests must verify up/down round-trip.
- All NDJSON artifacts must validate.
