# PI Memory

Cross-session agent memory daemon built in Rust with PostgreSQL and pgvector.

## Status

- **Contract:** Approved
- **Confidence:** 97/100
- **Phases:** 6

## Quick Links

- [Contract](contract.md)
- [Phases](phases.md)
- [Spec Index](spec/spec-index.md)

## Architecture

```text
AI agent / coding assistant
  ↓
MCP tools
  ↓
Rust MCP adapter
  ↓
Memory core
  ↓
PostgreSQL + pgvector
```

## Repository

```
agent-memory/
  crates/
    memory-core/       # Pure business logic
    memory-db/         # PostgreSQL + pgvector persistence
    memory-mcp/        # MCP server
    memory-daemon/     # Background workers
    memory-cli/        # CLI
    memory-api/        # HTTP API
    memory-providers/  # LLM/embedding integrations
    memory-tests/      # Integration tests
```

## Agent Artifacts

Machine-readable planning artifacts live under `.agent/contracts/pi-memory/`.
