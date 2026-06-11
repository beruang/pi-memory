# Phase 4: MCP Server Tools and Transport

**Depends on:** Phase 1 (core types), Phase 2 (database repositories)
**Risk:** high — primary agent interface; tool schema stability matters
**Value:** Agents can interact with the memory system through standardized MCP tools.

## Purpose

Implement `memory-mcp` with MCP server startup, 11 tool definitions, input/output schemas, and transport handling. The MCP layer must validate inputs, enforce authorization, and delegate to core services — no business logic in tool handlers.

## Deliverables

- MCP server initialization over stdio transport.
- Tool `memory.recall` — task-aware hybrid recall with token budget.
- Tool `memory.search` — explicit hybrid memory search.
- Tool `memory.get` — fetch full observation with evidence.
- Tool `memory.write` — write a source-backed observation.
- Tool `memory.update` — update an existing observation.
- Tool `memory.mark_obsolete` — mark observation obsolete with reason.
- Tool `memory.consolidate_session` — trigger session consolidation.
- Tool `memory.link_file` — link observation to a file path.
- Tool `memory.list_conflicts` — list unresolved conflicts.
- Tool `memory.resolve_conflict` — resolve a conflict (left_wins, right_wins, merge).
- Tool `memory.delete` — soft-delete an observation.
- Input validation and error mapping (MemoryError → safe MCP error responses).
- Authorization boundary enforcement (scope, project, user).
- Token budget enforcement in recall responses.

## Files

| File | Purpose |
|------|---------|
| `crates/memory-mcp/Cargo.toml` | MCP SDK, serde, tokio dependencies |
| `crates/memory-mcp/src/lib.rs` | Public re-exports |
| `crates/memory-mcp/src/server.rs` | MCP server startup, initialization |
| `crates/memory-mcp/src/tools.rs` | Tool handler implementations (delegating to core) |
| `crates/memory-mcp/src/schemas.rs` | Tool input/output JSON schemas |
| `crates/memory-mcp/src/transport.rs` | Stdio transport handling |

## Testing

- Unit tests for input validation (invalid scopes, missing required fields).
- Unit tests for error-to-MCP-response mapping (no secrets leaked).
- Integration tests: start MCP server, send tool calls, verify responses.
- Token budget tests: verify recall responses stay within budget.
- Authorization tests: cross-scope access denied.
