# Phase 1: Core Domain Models and Business Logic

**Depends on:** None
**Risk:** high — foundation for all subsequent phases
**Value:** Every other crate depends on these types and traits. Getting them right unlocks all downstream work.

## Purpose

Implement `memory-core` with all domain types, enums, traits, and pure business logic. This crate must have zero dependencies on PostgreSQL, MCP, HTTP, CLI, or specific LLM/embedding providers.

## Deliverables

- Observation struct with all fields (id, scope, project_id, user_id, organization_id, session_id, kind, summary, entities, files, commands, links, confidence, sensitivity, status, evidence, timestamps, supersession, metadata).
- EvidenceRef struct (id, observation_id, source_type, source_id, source_location, excerpt, created_at).
- MemoryScope enum (Session, Project, User, Organization).
- MemoryKind enum (Decision, Fact, Constraint, Preference, Procedure, ImplementationDetail, Bug, Fix, FailedAttempt, Todo, OpenQuestion, Dependency, Risk, Policy, ExternalReference).
- MemoryConfidence enum (Low, Medium, High).
- MemorySensitivity enum (Public, Internal, Private, Secret).
- MemoryStatus enum (Active, Unconfirmed, Superseded, Obsolete, Conflicted, Deleted).
- EvidenceSourceType enum (Message, ToolCall, File, Terminal, Commit, Issue, PullRequest, Document, Web, ManualEntry).
- MemoryError enum with Display implementations.
- EmbeddingProvider trait (async, returns `Vec<f32>`).
- ConsolidationProvider trait (async, takes ConsolidationInput, returns `Vec<CandidateObservation>`).
- Privacy filtering logic: secret detection via regex patterns, private block stripping, sensitivity classification.
- Conflict detection logic: entity+property+value matching, file+assumption matching, command+result matching.
- Ranking/scoring logic: the weighted scoring model (vector 45%, keyword 30%, confidence 15%, recency 10%).
- Memory lifecycle state machine: valid transitions between statuses.
- ConsolidationInput, ConsolidationOutput, CandidateObservation types.

## Files

| File | Purpose |
|------|---------|
| `crates/memory-core/Cargo.toml` | Crate manifest with minimal dependencies |
| `crates/memory-core/src/lib.rs` | Public re-exports |
| `crates/memory-core/src/observation.rs` | Observation struct |
| `crates/memory-core/src/evidence.rs` | EvidenceRef, EvidenceSourceType |
| `crates/memory-core/src/event.rs` | Session event types |
| `crates/memory-core/src/consolidation.rs` | ConsolidationInput/Output, CandidateObservation, ConsolidationProvider trait |
| `crates/memory-core/src/recall.rs` | Recall input/output types, token budget config |
| `crates/memory-core/src/ranking.rs` | Scoring model, weight configuration |
| `crates/memory-core/src/privacy.rs` | Secret detection, private block stripping, sensitivity classification |
| `crates/memory-core/src/conflict.rs` | Conflict detection logic, conflict categories |
| `crates/memory-core/src/lifecycle.rs` | Status state machine, valid transitions |
| `crates/memory-core/src/errors.rs` | MemoryError enum |

## Testing

- Unit tests for all enum Display/FromStr implementations.
- Unit tests for privacy filtering (known secret patterns, private block removal).
- Unit tests for conflict detection (positive and negative cases).
- Unit tests for lifecycle state machine (valid and invalid transitions).
- Unit tests for ranking/scoring model.
