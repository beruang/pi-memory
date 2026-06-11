# Spec Phase 4: MCP Server Tools and Transport

## Phase Goal

Implement `memory-mcp` with MCP server initialization, 11 tool definitions, input/output JSON schemas, and stdio transport. The MCP layer validates, authorizes, and delegates — no business logic in tool handlers.

## Dependencies

- Requires: Phase 1 (core types), Phase 2 (database repositories)
- Produces: `crates/memory-mcp/`

## Existing Code References

- Pattern to follow: MCP SDK server pattern with tool registration and handler functions.
- Test pattern: Start server on test transport, send JSON-RPC requests, verify responses.
- Config pattern: MCP transport from config (stdio default).
- Reference: `spec.md` section 19 (MCP Interface), section 19.2 (MCP Tools).

## Technical Approach

Use the Rust MCP SDK to define a server with tool capabilities. Each tool handler:
1. Validates input against JSON schema.
2. Checks authorization (scope, project, user boundaries).
3. Calls core service (from memory-db repository or memory-core logic).
4. Maps result to compact JSON response.
5. Maps errors to safe MCP error responses (no secrets, SQL, or internals).

**Key design decisions:**
- Tools are defined with JSON Schema for inputs.
- Token budget enforcement happens in `memory.recall` by truncating results.
- MCP transport is stdio by default; HTTP/SSE can be added later.
- No embedding or consolidation logic in MCP — those are daemon jobs.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/memory-mcp/Cargo.toml` | MCP SDK, serde, serde_json, tokio, memory-core, memory-db |
| `crates/memory-mcp/src/lib.rs` | Re-exports, server builder |
| `crates/memory-mcp/src/server.rs` | MCP server initialization, tool registration |
| `crates/memory-mcp/src/tools.rs` | Tool handler implementations (11 tools) |
| `crates/memory-mcp/src/schemas.rs` | JSON schemas for all tool inputs and outputs |
| `crates/memory-mcp/src/transport.rs` | Stdio transport setup |

### Modified Files

- `Cargo.toml` (workspace root) — add `memory-mcp` to workspace members.

## Implementation Steps

1. Create `crates/memory-mcp/Cargo.toml` with MCP SDK, serde, serde_json, tokio, memory-core, memory-db.
2. Implement `src/schemas.rs` — define input/output JSON schemas for all 11 tools per spec section 19.2.
3. Implement `src/server.rs` — `MemoryMcpServer::new(repos, config)` with tool registration and initialization.
4. Implement tool handlers in `src/tools.rs`:
   - `memory.recall` — task-aware recall: parse task, call hybrid search, enforce token budget, return compact results.
   - `memory.search` — explicit search: parse query, call hybrid search, return results.
   - `memory.get` — fetch by ID with evidence count.
   - `memory.write` — validate sensitivity != secret, require evidence, write observation.
   - `memory.update` — fetch existing, validate transition, apply update.
   - `memory.mark_obsolete` — set status to obsolete with reason.
   - `memory.consolidate_session` — trigger consolidation (delegates to daemon or runs inline).
   - `memory.link_file` — insert observation_files row.
   - `memory.list_conflicts` — query open conflicts for project.
   - `memory.resolve_conflict` — update conflict status, update observation statuses.
   - `memory.delete` — soft-delete with reason.
5. Implement `src/transport.rs` — stdio server startup.
6. Wire `src/lib.rs`.

## Data / API / Interface Contract

Tool input schemas match spec section 19.2. Key types:

```rust
// memory.recall input
{
    "task": "debug failed auth middleware tests",
    "scope": "project",
    "project_id": "uuid",
    "files": ["apps/api/src/auth/middleware.ts"],
    "token_budget": 1200
}

// memory.recall output
{
    "memories": [
        {
            "id": "uuid",
            "kind": "failed_attempt",
            "summary": "...",
            "confidence": "high",
            "status": "active",
            "evidence_count": 2
        }
    ]
}
```

## Error Handling

All tool handlers catch errors and return safe MCP error responses:
- ObservationNotFound → "Observation not found" (with ID).
- SecretContentRejected → "Content rejected: potential secret detected."
- InvalidScope → "Invalid scope. Must be one of: session, project, user, organization."
- AuthorizationDenied → "Access denied for requested scope."
- Database errors → "Internal error" (details logged, not returned).

## Observability

- Logs: `tracing::info` for tool calls with session_id, tool name, latency. `tracing::debug` for result counts.
- Metrics: tool call count, error count, recall latency histogram, token budget utilization.
- Traces: span per tool call.

## Testing Requirements

### Unit Tests

- Input validation: reject missing required fields, invalid scopes, invalid UUIDs.
- Error mapping: verify MemoryError → MCP error response contains no internals.
- Token budget: verify recall truncates to budget (character-based estimate).

### Integration Tests

- Start MCP server with test database, call each tool, verify responses.
- `memory.write` + `memory.get` round-trip.
- `memory.write` + `memory.search` — written memory appears in search.
- `memory.write` with secret content → rejected.
- `memory.write` + `memory.update` + `memory.mark_obsolete` lifecycle.
- `memory.write` conflicting observation → conflict detection.
- Authorization: cross-scope access denied.

### Regression Tests

- Tool schema stability: verify input/output schemas match expected shape.
- Private block content absent from all tool responses.

## Validation Commands

```bash
cargo build -p memory-mcp
cargo test -p memory-mcp
cargo clippy -p memory-mcp -- -D warnings
```

## Acceptance Criteria

- [ ] All 11 MCP tools respond to valid requests.
- [ ] `memory.recall` enforces token budget (character count ≤ budget × 4).
- [ ] `memory.write` rejects observations with sensitivity = "secret".
- [ ] `memory.write` rejects observations with no evidence and no user confirmation.
- [ ] `memory.search` returns results ordered by hybrid score.
- [ ] `memory.get` returns full observation with evidence references.
- [ ] `memory.resolve_conflict` updates both conflict and observation statuses.
- [ ] Error responses never contain SQL, connection strings, or secrets.
- [ ] Non-existent observation ID returns "not found" error.
- [ ] Cross-scope access is denied.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| MCP SDK API instability | medium | Pin SDK version; wrap SDK types behind thin adapter if needed |
| Token budget estimation inaccurate across models | low | Document as approximate; use character count / 4 as token estimate |
| Concurrent MCP sessions sharing database | medium | Use connection pool; test with concurrent tool calls |
