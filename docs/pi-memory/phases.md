# Phases: PI Memory

## Summary

The implementation is split into 6 phases following the crate dependency graph. Each phase produces one or more crates, is independently testable, and unlocks the next phase.

## Dependency Graph

```text
Phase 1: Core domain models
  ↓
Phase 2: Database persistence
  ↓
Phase 3: Provider layer
  ↓
Phase 4: MCP server
  ↓
Phase 5: Runtime infrastructure
  ↓
Phase 6: Integration and quality
```

## Phase Index

| ID | Title | Depends On | Spec | Risk | Status |
|---|---|---|---|---|---|
| phase-1 | Core domain models and business logic | — | spec/spec-phase-1.md | high | Draft |
| phase-2 | Database schema, migrations, repositories | phase-1 | spec/spec-phase-2.md | high | Draft |
| phase-3 | Provider abstractions and implementations | phase-1 | spec/spec-phase-3.md | medium | Draft |
| phase-4 | MCP server tools and transport | phase-1, phase-2 | spec/spec-phase-4.md | high | Draft |
| phase-5 | Daemon, CLI, and HTTP API | phase-1, phase-2, phase-3 | spec/spec-phase-5.md | medium | Draft |
| phase-6 | Integration tests, CI, and quality gates | phase-1–5 | spec/spec-phase-6.md | medium | Draft |

## Parallelization Notes

- Phase 2 and Phase 3 can run in parallel after Phase 1 completes (both depend only on core types).
- Phase 4 and Phase 5 can partially overlap: Phase 5 CLI/daemon scaffolding can start while Phase 4 MCP tools are being implemented.
- Phase 6 must run after all other phases are complete.

## Shared File Risks

- `crates/memory-core/src/lib.rs` — re-exported by all phases; changes must be coordinated.
- `crates/memory-db/src/lib.rs` — Phase 2 defines the schema; Phase 4 and Phase 5 consume it.
- `Cargo.toml` (workspace root) — every phase adds crate entries; merge sequentially.

Mitigation: Phases commit to the workspace sequentially. Each phase adds its crates and the next phase builds on the committed state.

## Per-Phase Detail Files

- [Phase 1: Core domain models](phases/phase-1.md)
- [Phase 2: Database persistence](phases/phase-2.md)
- [Phase 3: Provider layer](phases/phase-3.md)
- [Phase 4: MCP server](phases/phase-4.md)
- [Phase 5: Runtime infrastructure](phases/phase-5.md)
- [Phase 6: Integration and quality](phases/phase-6.md)
