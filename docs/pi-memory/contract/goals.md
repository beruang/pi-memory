# Goals

## Primary Goals

1. **Preserve useful continuity across sessions.** Agents must recall relevant prior context without manual prompting.
2. **Reduce repeated discovery work.** Known facts, constraints, decisions, and failed attempts must be retrievable.
3. **Capture durable project knowledge.** Architectural decisions, rationale, constraints, procedures, bugs, fixes, risks, policies, and unresolved work must survive session boundaries.
4. **Store memory as structured observations, not raw transcripts.** Every durable memory must have kind, confidence, sensitivity, evidence, and status.
5. **Attach provenance to every durable memory item.** Evidence references must link memories to their source.
6. **Retrieve only memory relevant to the current task.** Hybrid retrieval combining structured filters, full-text search, vector similarity, confidence, recency, and evidence scoring.
7. **Prevent sensitive information from being stored unintentionally.** Secret scanning, private blocks, and sensitivity classification must run before persistence.
8. **Allow users to inspect, edit, confirm, expire, obsolete, or delete memory.** Memory is context, not unquestionable truth.
9. **Detect conflicts between new and existing memories.** Contradictory observations must be flagged, not silently merged.
10. **Keep context injection token-efficient.** Memory injection must be bounded by configurable token budgets.

## Secondary Goals

1. Provide a local web review UI for memory inspection and management.
2. Support both automatic retrieval and explicit agent-driven memory search.
3. Expose the memory engine through multiple interfaces: MCP, CLI, HTTP API.
4. Run as a single Rust binary with embedded daemon, workers, and optional UI.

## Measurable Targets

| Goal | Metric | Target |
|------|--------|--------|
| Recall latency | p95 hybrid search + scoring | < 500ms |
| Secret rejection | False negative rate on standard secret patterns | 0% |
| Token budget enforcement | Max tokens injected per recall call | Configurable, default 1200 |
| Memory durability | Observation writes with evidence | 100% of writes must have >= 1 evidence ref or explicit user confirmation |
| Conflict detection | Recall on known-conflict pairs | 100% detection rate |

## Priority Order

1. Structured observation model with provenance
2. PostgreSQL persistence with pgvector
3. Hybrid retrieval (full-text + vector + structured filters)
4. Privacy filtering and secret detection
5. MCP server interface
6. Conflict detection
7. CLI and daemon
8. Review UI
9. Provider abstractions (embeddings, consolidation)
