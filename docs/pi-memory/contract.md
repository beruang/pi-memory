# Contract: PI Memory

**Status:** Approved
**Created:** 2026-06-12
**Updated:** 2026-06-12
**Confidence Score:** 97/100
**Project Slug:** pi-memory
**Source Input:** spec.md — comprehensive specification for a Rust-based cross-session agent memory daemon using PostgreSQL with pgvector, exposed via MCP.
**Target Repository:** /Volumes/Workspace/rnd/workflow/mcp/pi-memory

## Summary

Build a Rust-based cross-session agent memory daemon that stores structured, source-backed observations in PostgreSQL with pgvector, retrieves relevant memories via hybrid search (vector + keyword + structured filters), exposes MCP tools for AI agents, and enforces privacy through secret scanning and sensitivity classification.

## Confidence Summary

| Dimension | Score | Reason |
|---|---:|---|
| Problem Clarity | 19/20 | Problem is precisely defined: agents lose cross-session context, rediscover facts, repeat failures, forget constraints. Pain and impact are explicit. |
| Goal Definition | 19/20 | 10 primary goals, 4 secondary goals, measurable targets with latency/budget/rejection metrics. |
| Success Criteria | 19/20 | 20+ binary acceptance criteria, artifact checks, validation commands, and an explicit done/not-done boundary. |
| Scope Boundaries | 20/20 | In-scope and out-of-scope lists are comprehensive. Explicitly deferred items are listed with reasons. |
| Consistency | 20/20 | No contradictions across 39 sections. Architecture is internally consistent: Rust/tokio/sqlx/pgvector, thin MCP, independent core, privacy-first. |

## Problem Statement

AI coding agents lose critical context between sessions. They rediscover facts, repeat failed attempts, forget project constraints, and ignore stable user preferences. Current workarounds (CLAUDE.md files, re-explaining, raw transcripts) are manual, stale, noisy, or unsafe.

Detailed version: `contract/problem.md`

## Goals

1. Preserve useful continuity across sessions.
2. Reduce repeated discovery work.
3. Capture durable project knowledge (decisions, constraints, bugs, fixes, procedures, risks).
4. Store memory as structured observations, not raw transcripts.
5. Attach provenance to every durable memory.
6. Retrieve only task-relevant memory via hybrid search.
7. Prevent sensitive information from being stored.
8. Allow users to inspect, edit, confirm, obsolete, or delete memory.
9. Detect conflicts between new and existing memories.
10. Keep context injection token-efficient.

Detailed version: `contract/goals.md`

## Success Criteria

- Structured observations with kind, confidence, sensitivity, status, and evidence.
- PostgreSQL canonical store; pgvector for semantic recall only.
- Hybrid retrieval combining vector, keyword, structured filters, and scoring.
- 11 MCP tools for agents.
- Secret content rejected before persistence.
- Private blocks excluded from all persistence paths.
- Conflicted memory never authoritative without resolution.
- Token budgets enforced per recall type.
- All lint, format, test, audit, and deny checks pass.

Detailed version: `contract/success-criteria.md`

## Scope Boundaries

**In scope:** Observation model, PostgreSQL+pgvector persistence, hybrid retrieval, privacy filtering, conflict detection, memory lifecycle, MCP server (11 tools), daemon with background workers, CLI (8 subcommands), HTTP API, review UI, provider traits, Context7 integration, token-budgeted recall, audit log, event buffer, CI pipeline.

**Out of scope:** Raw transcript storage, secrets storage, pgvector as canonical source, auto-merging contradictory memories, hard-coding any LLM/embedding provider, business logic in MCP handlers.

Detailed version: `contract/scope.md`

## Constraints

- Rust workspace with 8 crates, strict dependency boundaries.
- `memory-core` must not depend on PostgreSQL, MCP, HTTP, CLI, or specific providers.
- No business logic in MCP handlers. No direct SQL in MCP handlers.
- LLM/embedding providers behind traits.
- Token budgets: 700–1800 tokens per recall type.
- No `#[allow(...)]` without justification.
- Secrets never persisted. Private blocks stripped before indexing.
- Soft deletion by default.

Detailed version: `contract/constraints.md`

## Assumptions

Key assumptions: PostgreSQL+pgvector available in dev/CI, pgvector HNSW sufficient for <100K observations, single embedding model sufficient initially, MCP stdio is the primary interface, regex-based secret detection covers common patterns, consolidation is LLM-driven behind a trait.

Detailed version: `contract/assumptions.md`

## Decisions

Key decisions: Rust implementation, PostgreSQL+pgvector, separate embedding storage, MCP as thin adapter, hybrid scoring weights (45/30/15/10), evidence required for all durable memory, soft-delete default, provider traits, 8-crate workspace, temporary event buffer.

Detailed version: `contract/decisions.md`

## Risks

Top risks: secret detection false negatives (high severity), consolidation LLM hallucination (high likelihood), pgvector index build time (medium), embedding latency backpressure (medium), conflict detection false positives (medium).

Detailed version: `contract/risks.md`

## Approval

- Status: Approved
- Approved By: user
- Approved At: 2026-06-12
