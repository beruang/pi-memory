# Assumptions

| # | Assumption | Confidence Impact | What Would Invalidate |
|---|-----------|-------------------|----------------------|
| !1 | PostgreSQL with pgvector, pgcrypto, and pg_trgm extensions is available in both dev and CI environments. | High — drives the entire persistence architecture. | Would require an alternative vector store or fallback to in-memory search. |
| !2 | pgvector HNSW index performance is sufficient for the expected observation volume (< 100K observations). | Medium — drives index strategy. | Would require IVF flat indexes or external vector store. |
| !3 | A single embedding model is sufficient for initial semantic search quality. | Medium — drives embedding storage design (model-per-row already supported). | Would need multi-model search with cross-model ranking. |
| !4 | The MCP protocol over stdio is the primary agent interface; HTTP/SSE transport can follow. | Low — drives transport priority. | Would change MCP transport implementation order. |
| !5 | Secret scanning via regex patterns covers the most common secret shapes (API keys, tokens, JWTs, private keys, connection strings). | Medium — drives privacy approach. | Would require ML-based secret detection or external secret scanner integration. |
| !6 | Consolidation (event → observation) is driven by an LLM provider behind the ConsolidationProvider trait. | Medium — drives consolidation design. | Would need a rule-based or template-based consolidation fallback. |
| !7 | The review UI is a local-only web interface with no authentication. | Low — drives API design. | Would require auth middleware on the HTTP API. |
| !8 | A single project context is active at a time for the MCP server. | Low — drives scope filtering. | Would need multi-project context switching in MCP tools. |

## Open Questions

1. Whether the consolidation worker should run automatically on session end or only on explicit `memory.consolidate_session` call.
2. Whether embedding regeneration on summary change should be synchronous or queued.
3. Exact HNSW index parameters (`m`, `ef_construction`) for the target observation volume.
4. Whether the review UI should be a separate binary or embedded in `agent-memory serve`.

## Unknowns

- Exact observation volume per project for performance tuning.
- Preferred embedding model at deploy time (design supports any via trait).
- Whether organization-scope memory will be used in the initial deployment.
