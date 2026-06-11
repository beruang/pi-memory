# Problem

## Current Pain

AI coding agents operating across multiple sessions lose critical context. Each new session starts cold. The agent rediscovers the same facts, repeats failed attempts, forgets project constraints, loses architectural rationale, and ignores stable user preferences.

## Affected Users

- **AI coding agents** — Cannot access prior-session knowledge without manual prompting.
- **Developers** — Must re-explain project context, preferences, constraints, and decisions to the agent in every session.
- **Teams** — Lose institutional knowledge about why architectural decisions were made, what approaches failed, and what constraints exist.

## Current Workaround

Today, agents rely on:
- Reading files from scratch each session (slow, misses rationale).
- CLAUDE.md or similar project context files (manual, stale).
- User re-explaining context verbally (repetitive, incomplete).
- Conversation transcripts (noisy, unstructured, unsafe to persist).

## Impact

- **Time:** Repeated discovery work per session.
- **Quality:** Agents repeat known-failed approaches.
- **Trust:** Agents forget user preferences and constraints.
- **Risk:** Sensitive data may leak into unstructured transcript storage.
- **Continuity:** No durable link between decisions made in one session and work in the next.

## Why Now

Agentic coding is moving from single-session interactions to multi-session collaboration. Without structured memory, agents cannot build on prior work, learn from failures, or respect evolving constraints. The cost of cold-start sessions grows with project complexity and session count.
