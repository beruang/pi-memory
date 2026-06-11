# Decisions

| # | Decision | Reason | Alternatives Considered | Impact |
|---|---------|--------|------------------------|--------|
| 1 | Use Rust as the implementation language. | Single-binary distribution, strong typing for memory lifecycle states, memory safety, async runtime, cross-platform. | Go (weaker type system for state machines), Python (no single binary, GIL), TypeScript (no native daemon). | Drives entire toolchain, crate structure, and distribution model. |
| 2 | Use PostgreSQL with pgvector as the canonical store and vector index. | Single database for structured + vector data, mature, well-supported, transactional. | SQLite + external vector store, standalone vector DB (Pinecone, Weaviate, Qdrant). | Simplifies deployment (one DB), enables hybrid queries in SQL, avoids syncing two stores. |
| 3 | Store embeddings separately from observations with model + dimensions columns. | Supports multiple embedding models, dimension changes, and model migration without re-creating the observations table. | Store embedding inline in observations table. | Enables model evolution and A/B testing of embedding quality. |
| 4 | Use MCP as the agent-facing protocol layer, not the core architecture. | Keeps the memory engine reusable across interfaces (CLI, HTTP, future protocols). | Embed MCP directly in core logic, make MCP the only interface. | Prevents vendor lock-in to MCP; core remains testable without MCP harness. |
| 5 | Use hybrid retrieval scoring (vector 45%, keyword 30%, confidence 15%, recency 10%) as the default. | Balances semantic relevance with exact-match precision and freshness. | Pure vector search, pure keyword search, LLM-reranked results. | Defines the retrieval quality profile; weights are configurable. |
| 6 | Require at least one evidence reference or explicit user confirmation for every durable memory. | Prevents ungrounded observations from polluting the memory store. | Allow zero-evidence observations, require evidence only above a confidence threshold. | Ensures every memory can be traced to its source. |
| 7 | Soft-delete by default with `status = deleted`; support hard deletion for privacy/compliance. | Preserves audit trail while allowing removal when legally required. | Hard delete only, soft delete only. | Balances accountability with privacy rights. |
| 8 | Put LLM and embedding providers behind traits with async methods. | Avoids hard-coding any provider; enables local and cloud providers. | Hard-code OpenAI as default, add traits later. | Keeps the core provider-agnostic from day one. |
| 9 | Use a Rust workspace with 8 crates separated by concern. | Clear dependency boundaries, faster incremental builds, enforceable architecture rules. | Single crate with modules, 3 crates (core, db, mcp). | Defines the repository structure and crate dependency graph. |
| 10 | Capture session events temporarily with short, configurable retention. | Enables consolidation without storing raw events indefinitely. | No event buffer (consolidate directly), permanent event storage. | Limits privacy risk from raw event data while enabling consolidation. |

## Superseded Decisions

None — initial contract.

## Rejected Alternatives

| Alternative | Rejected Because |
|-------------|-----------------|
| SQLite + external vector DB | Two stores to manage, no transactional hybrid queries, higher operational complexity. |
| Go implementation | Weaker type system for the state-machine-like memory lifecycle; harder to enforce correct state transitions at compile time. |
| Inline embeddings in observations table | Locks the system to one embedding model; prevents dimension migration. |
| MCP as the only interface | Makes core untestable without MCP harness; blocks CLI and HTTP use cases. |
| Pure vector search without structured filters | Embeddings alone miss exact keyword matches (commands, file paths) and provide no provenance filtering. |
