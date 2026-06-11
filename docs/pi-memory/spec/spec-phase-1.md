# Spec Phase 1: Core Domain Models and Business Logic

## Phase Goal

Implement `memory-core` crate with all domain types, enums, traits, and pure business logic. Zero dependencies on PostgreSQL, MCP, HTTP, CLI, or specific providers.

## Dependencies

- Requires: None
- Produces: `crates/memory-core/`

## Existing Code References

- Pattern to follow: Standard Rust library crate with `#[derive]` macros, `thiserror`, `serde`, `async-trait`.
- Test pattern: Inline `#[cfg(test)] mod tests` with unit tests per module.
- Config pattern: Types only — no configuration in this crate.
- Reference: `spec.md` sections 12 (Observation model), 13 (Evidence), 14 (Observation Kinds), 21 (Privacy), 23 (Conflict), 24 (Lifecycle).

## Technical Approach

Implement as a pure Rust library with minimal dependencies (`serde`, `thiserror`, `async-trait`, `uuid`, `time`). All domain logic is in pure functions — no I/O, no database, no network.

**Key design decisions:**
- Use `#[derive]` for common traits (Debug, Clone, Serialize, Deserialize).
- Use `sqlx::Type` derive on enums so `memory-db` can reuse the enum definitions directly.
- Use `thiserror` for domain errors with `#[from]` for common conversions.
- Use `#[async_trait::async_trait]` for provider traits to support async implementations.
- Privacy filtering is pure: `fn scan_for_secrets(input: &str) -> Vec<SecretMatch>`.
- Conflict detection is pure: `fn detect_conflicts(new: &Observation, existing: &[Observation]) -> Vec<Conflict>`.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/memory-core/Cargo.toml` | serde, thiserror, async-trait, uuid, time, regex |
| `crates/memory-core/src/lib.rs` | Public re-exports of all modules |
| `crates/memory-core/src/observation.rs` | Observation struct with all fields |
| `crates/memory-core/src/evidence.rs` | EvidenceRef struct, EvidenceSourceType enum |
| `crates/memory-core/src/event.rs` | SessionEvent struct, event types |
| `crates/memory-core/src/consolidation.rs` | ConsolidationInput/Output, CandidateObservation, ConsolidationProvider trait, EmbeddingProvider trait |
| `crates/memory-core/src/recall.rs` | RecallRequest, RecallResponse, token budget config |
| `crates/memory-core/src/ranking.rs` | Scoring model with configurable weights |
| `crates/memory-core/src/privacy.rs` | SecretPattern, scan_for_secrets, strip_private_blocks, classify_sensitivity |
| `crates/memory-core/src/conflict.rs` | ConflictCategory, detect_conflicts, ConflictRecord |
| `crates/memory-core/src/lifecycle.rs` | Status state machine with valid_transitions function |
| `crates/memory-core/src/errors.rs` | MemoryError enum with Display and thiserror derives |

### Modified Files

None — new crate.

## Implementation Steps

1. Create `crates/memory-core/Cargo.toml` with dependencies: `serde = { version = "1", features = ["derive"] }`, `thiserror`, `async-trait`, `uuid = { version = "1", features = ["v4", "serde"] }`, `time = { version = "0.3", features = ["serde"] }`, `regex`, `serde_json`.
2. Implement `src/errors.rs` — MemoryError enum per spec section 28.
3. Implement `src/observation.rs` — Observation struct per spec section 12.1, with all enums (MemoryScope, MemoryKind, MemoryConfidence, MemorySensitivity, MemoryStatus) per sections 12.2–12.6.
4. Implement `src/evidence.rs` — EvidenceRef per spec section 13.1, EvidenceSourceType per section 13.2.
5. Implement `src/event.rs` — SessionEvent types per spec section 20.
6. Implement `src/consolidation.rs` — ConsolidationInput, ConsolidationOutput, CandidateObservation, EmbeddingProvider trait, ConsolidationProvider trait per spec section 6.
7. Implement `src/recall.rs` — RecallRequest, RecallResponse, token budget configuration per spec sections 17–18.
8. Implement `src/privacy.rs` — secret scanning regex patterns per spec section 21.3, private block stripping per section 21.2, sensitivity classification.
9. Implement `src/conflict.rs` — conflict categories per spec section 23, detection function.
10. Implement `src/lifecycle.rs` — status state machine with valid transitions per spec section 24.
11. Implement `src/ranking.rs` — weighted scoring model per spec section 16.1.
12. Wire `src/lib.rs` to re-export all public types.

## Data / API / Interface Contract

```rust
// Core types exported from memory-core
pub use observation::{Observation, MemoryScope, MemoryKind, MemoryConfidence, MemorySensitivity, MemoryStatus};
pub use evidence::{EvidenceRef, EvidenceSourceType};
pub use errors::MemoryError;
pub use privacy::{scan_for_secrets, strip_private_blocks, classify_sensitivity, SecretMatch, PrivateBlockRange};
pub use conflict::{detect_conflicts, ConflictCategory, ConflictRecord};
pub use lifecycle::valid_transition;
pub use ranking::{score_observations, ScoreWeights};
pub use consolidation::{EmbeddingProvider, ConsolidationProvider, ConsolidationInput, ConsolidationOutput, CandidateObservation};
pub use recall::{RecallRequest, RecallResponse, TokenBudget};
```

## Error Handling

- `MemoryError::InvalidScope` — invalid scope string.
- `MemoryError::SecretContentRejected` — content classified as secret.
- `MemoryError::ConflictDetected` — new observation conflicts with existing.
- `MemoryError::InvalidStatusTransition` — illegal lifecycle move.
- `MemoryError::MissingEvidence` — observation written without evidence.

## Observability

- Logs: `tracing::debug` for privacy scan results (count, not content).
- Metrics: None at this layer (no runtime).
- Traces: None at this layer.

## Testing Requirements

### Unit Tests

- All enum Display/FromStr round-trips.
- `valid_transition` — test every allowed transition, test every disallowed transition.
- `scan_for_secrets` — known API key patterns, JWTs, private keys, connection strings, false positives.
- `strip_private_blocks` — nested blocks, unclosed blocks, empty blocks.
- `classify_sensitivity` — public, internal, private, secret boundary cases.
- `detect_conflicts` — same entity + same property + incompatible value → conflict. Same entity + different property → no conflict.
- `score_observations` — verify weight distribution sums correctly.

### Integration Tests

None — this crate has no external dependencies.

### Regression Tests

- Secret pattern tests with real-looking (but fake) credentials.

## Validation Commands

```bash
cargo build -p memory-core
cargo test -p memory-core
cargo clippy -p memory-core -- -D warnings
cargo fmt --check -p memory-core
```

## Acceptance Criteria

- [ ] `crates/memory-core/` compiles with zero warnings.
- [ ] All unit tests pass.
- [ ] Crate has no dependency on sqlx, axum, tokio, clap, or any provider-specific crate.
- [ ] All enums derive `sqlx::Type` for reuse by memory-db.
- [ ] Privacy scanner detects all patterns listed in spec section 21.3.
- [ ] Private blocks are fully stripped (content between tags removed).
- [ ] Conflict detection covers all categories in spec section 23.
- [ ] Lifecycle state machine rejects invalid transitions.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Enum definitions drift from spec | medium | Cross-reference spec sections 12.2–12.6 during implementation |
| Secret patterns miss novel formats | high | Use conservative patterns; add entropy-based detection as second pass |
| Provider traits too narrow for real implementations | medium | Design traits for one method initially; extend as providers demand |
