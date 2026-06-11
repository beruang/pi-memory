# Spec Index: PI Memory

## Summary

6 implementation specs, one per phase. Each spec is independently executable and testable. Specs follow the dependency graph defined in `phases.md`.

## Specs

| Spec | Title | Depends On | Purpose | Status |
|---|---|---|---|---|
| spec-phase-1.md | Core domain models and business logic | — | All domain types, enums, traits, privacy, conflict, ranking | Draft |
| spec-phase-2.md | Database schema, migrations, repositories | Phase 1 | PostgreSQL+pgvector persistence, hybrid search SQL | Draft |
| spec-phase-3.md | Provider abstractions and implementations | Phase 1 | Embedding and consolidation provider implementations | Draft |
| spec-phase-4.md | MCP server tools and transport | Phase 1, Phase 2 | 11 MCP tools, transport, schemas | Draft |
| spec-phase-5.md | Daemon, CLI, and HTTP API | Phase 1, Phase 2, Phase 3 | Background workers, CLI, HTTP API, review UI | Draft |
| spec-phase-6.md | Integration tests, CI, quality gates | Phase 1–5 | Integration tests, CI pipeline, deny.toml | Draft |

## Recommended Reading Order

1. `../contract.md` — what and why
2. `../phases.md` — breakdown and dependencies
3. `spec-phase-1.md` through `spec-phase-6.md` — implementation instructions
4. `../../spec.md` — original specification for reference

## Agent Loading Guidance

Implementation agents should start with:

1. `.agent/contracts/pi-memory/manifest.json`
2. `.agent/contracts/pi-memory/specs.index.ndjson`
3. The specific phase spec assigned to them
