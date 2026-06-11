# Risks

## Risk Register

| # | Risk | Severity | Likelihood | Mitigation | Owner |
|---|------|----------|------------|------------|-------|
| 1 | pgvector HNSW index build time exceeds acceptable thresholds for large observation sets. | medium | possible | Benchmark with 100K+ synthetic observations; fall back to IVF flat indexes if needed. | memory-db author |
| 2 | Secret detection via regex produces false negatives for novel secret formats. | high | possible | Combine regex with entropy-based detection; log all rejections for audit; allow users to mark content as secret manually. | memory-core author |
| 3 | Consolidation LLM produces hallucinated or low-quality observations from session events. | high | likely | Require evidence attachment for every generated observation; keep confidence at `low` or `unconfirmed` for auto-consolidated memories; always surface for user review. | memory-providers author |
| 4 | Embedding generation latency causes consolidation pipeline backpressure. | medium | possible | Queue embeddings separately; generate embeddings asynchronously after durable write; monitor embedding queue depth. | memory-daemon author |
| 5 | Token budget enforcement is approximate and varies by model/tokenizer. | low | likely | Use configurable character-based estimates as proxy; document that budgets are approximate; allow per-scope budget tuning. | memory-mcp author |
| 6 | Conflict detection produces false positives on semantically compatible observations. | medium | possible | Use conservative conflict criteria (same entity + same property + incompatible value); allow users to dismiss false conflicts; track false-positive rate. | memory-core author |
| 7 | MCP stdio transport fails under high concurrency or large payloads. | medium | unlikely | Keep MCP responses compact; enforce token budgets; support streaming responses for large result sets; test with concurrent clients. | memory-mcp author |
| 8 | Review UI exposes debugging information or internal state to unintended viewers (local network). | low | possible | Bind to localhost only by default; warn on 0.0.0.0 bind; do not expose secret or private content (system never stores it). | memory-api author |

## Watch List

- Observation count growth rate — monitor to validate HNSW index parameter choices.
- Embedding model quality drift — periodically evaluate recall precision with sample queries.
- Secret detection false-positive rate — too-aggressive filtering loses useful context.
- Consolidation LLM cost — track tokens consumed per consolidation run.

## Kill Switches

- If secret detection fails in production (secret observed in stored memory), disable automatic consolidation and switch to manual-only writes until the scanner is fixed.
- If pgvector query latency exceeds 2s p95, disable vector search and fall back to keyword-only retrieval.
- If consolidation produces > 50% `low` confidence or `unconfirmed` observations, pause automatic consolidation and investigate provider quality.
- If memory database size grows beyond disk budget, trigger aggressive retention pruning and alert.
