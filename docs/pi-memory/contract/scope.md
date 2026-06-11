# Scope Boundaries

## In Scope

- Structured observation model (Observation, EvidenceRef, enums for scope, kind, confidence, sensitivity, status).
- PostgreSQL schema with pgvector extension, full-text search, and HNSW indexes.
- Hybrid retrieval combining vector similarity, keyword search, structured filters, and scoring.
- Privacy filtering: secret detection, private block removal, sensitivity classification.
- Conflict detection between new and existing observations.
- Memory lifecycle: active → unconfirmed → superseded/obsolete/conflicted/deleted.
- Supersession tracking with reason.
- MCP server exposing 11 tools over stdio transport.
- Background daemon with consolidation queue, embedding jobs, cleanup, and maintenance.
- CLI with subcommands: serve, mcp, migrate, search, recall, consolidate, inspect, ui.
- Local HTTP API for review UI and external integrations.
- Local web review UI for memory inspection, editing, conflict resolution, and export.
- Provider abstractions: EmbeddingProvider and ConsolidationProvider traits.
- Context7 integration for documentation lookup during implementation.
- Token-budgeted context injection per recall type.
- Audit log for all memory mutations.
- Event buffer for temporary session events with configurable retention.
- Configuration via file (TOML) and environment variables.
- Rust workspace with 8 crates: memory-core, memory-db, memory-mcp, memory-daemon, memory-cli, memory-api, memory-providers, memory-tests.
- CI pipeline with fmt, clippy, test, audit, deny, and PostgreSQL+pgvector provisioning.

## Out of Scope

- Storing raw chat transcripts as durable memory.
- Storing secrets, credentials, tokens, private keys, or `.env` contents.
- Using pgvector as the canonical source of truth.
- Using embeddings as a substitute for structured memory.
- Replacing project documentation, source control, issue tracking, or audit logs.
- Preserving hidden reasoning traces as durable memory.
- Auto-merging contradictory memories without evidence or confirmation.
- Hard-dependency on one LLM provider, embedding provider, editor, or agent client.
- Business logic directly inside MCP tool handlers.
- Full chat transcript archiving.

## Future Considerations

- Multi-user collaboration with shared organization memory.
- Memory export/import between projects.
- Plugin system for custom embedding or consolidation providers.
- Memory federation across multiple agent-memory instances.
- Integration with external issue trackers and documentation systems.
- Automatic memory decay based on access patterns.

## Explicitly Deferred

- Organization-level memory write controls (specified but deferred until multi-user support).
- Hard deletion compliance workflows (soft delete is implemented; hard delete deferred).
- Plugin/provider marketplace or discovery mechanism.
