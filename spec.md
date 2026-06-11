# PI Memory

## 1. Purpose

The system provides persistent, structured, source-backed memory for AI agents operating across multiple sessions.

Its purpose is to preserve useful continuity between sessions without preserving unnecessary history. The system helps an agent avoid rediscovering the same facts, repeating failed attempts, forgetting project constraints, losing architectural rationale, or ignoring stable user preferences.

The system is not a general transcript archive. It is a selective memory substrate that captures, filters, consolidates, retrieves, verifies, and updates durable observations.

The core principle is:

> Memory is not what the agent saw. Memory is what remains useful, grounded, safe, and relevant after the session ends.

## 2. Chosen Architecture

The system is implemented as a Rust-based memory service using PostgreSQL with pgvector.

The system exposes an MCP server as the primary interface for AI agents. The MCP layer must remain thin. The memory engine must remain independent of MCP so the core can be reused by CLI tools, HTTP APIs, local UIs, background workers, or future agent runtimes.

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

The system may also expose:

```text
Local CLI
Local HTTP API
Local web review UI
Background consolidation worker
Embedding worker
Maintenance jobs
```

## 3. Core Goals

The system must:

* Preserve useful continuity across sessions.
* Reduce repeated discovery work.
* Capture durable project knowledge.
* Capture architectural decisions and rationale.
* Capture known constraints, procedures, preferences, bugs, fixes, failed attempts, risks, policies, and unresolved work.
* Store memory as structured observations, not raw transcripts.
* Attach provenance to every durable memory item.
* Retrieve only memory relevant to the current task.
* Support semantic recall through pgvector.
* Support keyword recall through PostgreSQL full-text search.
* Support hybrid retrieval combining structured filters, full-text search, vector similarity, confidence, recency, evidence, and status.
* Prevent sensitive or private information from being stored unintentionally.
* Allow users to inspect, edit, confirm, expire, obsolete, or delete memory.
* Detect conflicts between new and existing memories.
* Support both automatic retrieval and explicit agent-driven memory search.
* Keep context injection token-efficient.
* Use Context7 during implementation when current library documentation is needed.
* Enforce linter, formatter, type checks, migrations checks, and tests.

## 4. Non-Goals

The system must not:

* Store every message, tool call, log, or terminal output as durable memory.
* Treat memory as inherently correct.
* Inject all available memory into every session.
* Store secrets, credentials, tokens, private keys, or private content.
* Use pgvector as the canonical source of truth.
* Use embeddings as a substitute for structured memory.
* Replace project documentation, source control, issue tracking, or audit logs.
* Preserve hidden reasoning traces as durable memory.
* Make old memories equally authoritative forever.
* Automatically merge contradictory memories without evidence or confirmation.
* Depend on one LLM provider, embedding provider, editor, or agent client.
* Put business logic directly inside MCP tool handlers.

## 5. Implementation Language

The system is implemented in Rust.

Rust is selected because the system is expected to behave as a reliable local or hosted daemon that handles potentially sensitive project context.

Rust is preferred for:

* Single-binary distribution.
* Long-running local daemon behavior.
* Strong typing of memory lifecycle states.
* Memory safety.
* Performance.
* Async event handling.
* Reliable background workers.
* CLI ergonomics.
* Explicit error handling.
* Cross-platform distribution.

## 6. Recommended Rust Stack

The implementation should use the following Rust ecosystem components unless there is a strong reason to replace them:

```text
Async runtime: tokio
Database: sqlx
Database backend: PostgreSQL
Vector search: pgvector
HTTP API: axum
CLI: clap
Serialization: serde
Logging/tracing: tracing, tracing-subscriber
Errors: thiserror for domain errors, anyhow for application boundaries
Configuration: figment or config
Migrations: sqlx migrate
UUID: uuid
Time: time or chrono
Secret scanning: regex plus custom detectors
Testing: cargo test, integration tests, testcontainers when needed
Formatting: rustfmt
Linting: clippy
Security audit: cargo audit
Dependency hygiene: cargo deny
```

The system should keep LLM and embedding providers behind traits.

```rust
#[async_trait::async_trait]
pub trait EmbeddingProvider {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, MemoryError>;
}

#[async_trait::async_trait]
pub trait ConsolidationProvider {
    async fn consolidate(
        &self,
        input: ConsolidationInput
    ) -> Result<Vec<CandidateObservation>, MemoryError>;
}
```

The core must not hard-code OpenAI, Anthropic, Ollama, Voyage, Cohere, local embeddings, or any other provider.

## 7. Product Shape

The system should be usable as a single binary.

Expected command shape:

```bash
agent-memory serve
agent-memory mcp
agent-memory migrate
agent-memory search "auth migration"
agent-memory recall --task "debug failed auth middleware tests"
agent-memory consolidate --session <session-id>
agent-memory inspect <observation-id>
agent-memory ui
```

### 7.1 `agent-memory serve`

Runs the memory daemon, local API, workers, and optional review UI.

### 7.2 `agent-memory mcp`

Runs the MCP server over stdio or supported MCP transport.

### 7.3 `agent-memory migrate`

Runs PostgreSQL migrations.

### 7.4 `agent-memory search`

Performs human-facing memory search.

### 7.5 `agent-memory recall`

Runs task-aware recall.

### 7.6 `agent-memory consolidate`

Runs manual or scheduled session consolidation.

### 7.7 `agent-memory inspect`

Displays one memory item with provenance.

### 7.8 `agent-memory ui`

Starts the local memory review interface.

## 8. Repository Structure

A Rust workspace is recommended.

```text
agent-memory/
  Cargo.toml

  crates/
    memory-core/
      src/
        lib.rs
        observation.rs
        evidence.rs
        event.rs
        consolidation.rs
        recall.rs
        ranking.rs
        privacy.rs
        conflict.rs
        lifecycle.rs
        errors.rs

    memory-db/
      src/
        lib.rs
        postgres.rs
        observations_repo.rs
        evidence_repo.rs
        embeddings_repo.rs
        search_repo.rs
        conflicts_repo.rs
        migrations.rs
      migrations/

    memory-mcp/
      src/
        lib.rs
        server.rs
        tools.rs
        schemas.rs
        transport.rs

    memory-daemon/
      src/
        lib.rs
        worker.rs
        queue.rs
        scheduler.rs

    memory-cli/
      src/
        main.rs
        commands.rs

    memory-api/
      src/
        lib.rs
        routes.rs
        handlers.rs

    memory-providers/
      src/
        lib.rs
        embeddings.rs
        consolidation.rs
        context7.rs

    memory-tests/
      tests/
        integration_memory.rs
        integration_search.rs
        integration_mcp.rs
```

## 9. Crate Boundaries

### 9.1 `memory-core`

Contains pure business logic.

It must not depend directly on:

* PostgreSQL
* MCP
* HTTP
* CLI
* Specific LLM providers
* Specific embedding providers

It owns:

* Observation model
* Evidence model
* Memory lifecycle rules
* Consolidation policy
* Recall policy
* Ranking logic
* Privacy filtering
* Conflict detection
* Domain errors

### 9.2 `memory-db`

Contains persistence and search implementation.

It owns:

* PostgreSQL connection handling
* SQL queries
* Migrations
* Repository implementations
* Full-text search
* pgvector search
* Hybrid retrieval SQL
* Audit log persistence
* Event buffer persistence

### 9.3 `memory-mcp`

Contains MCP protocol handling.

It owns:

* MCP server startup
* Tool definitions
* Tool input schemas
* Tool output schemas
* Transport handling
* Mapping MCP calls to core services

It must not own memory business logic.

### 9.4 `memory-daemon`

Contains background execution.

It owns:

* Consolidation queue
* Embedding jobs
* Cleanup jobs
* Retention jobs
* Conflict detection jobs
* Scheduled maintenance

### 9.5 `memory-cli`

Contains human-facing command-line tools.

### 9.6 `memory-api`

Contains optional local HTTP API for review UI and external integrations.

### 9.7 `memory-providers`

Contains integrations with embedding providers, consolidation providers, and documentation lookup tools such as Context7.

## 10. Conceptual Model

The memory lifecycle is:

```text
Observe → Consolidate → Recall → Verify → Update
```

### 10.1 Observe

The system captures relevant session events.

Observation capture does not mean durable memory storage. Captured events are temporary inputs for consolidation.

### 10.2 Consolidate

The system converts noisy session events into structured memory objects.

Consolidation includes:

* Private block removal
* Secret detection
* Redaction
* Sensitivity classification
* Importance scoring
* Candidate observation extraction
* Deduplication
* Contradiction detection
* Evidence attachment
* Confidence assignment
* Lifecycle assignment
* Durable write
* Index update
* Embedding generation

### 10.3 Recall

The system retrieves relevant memories for the current task, project, file, entity, user, or organization.

Recall must be staged and selective.

### 10.4 Verify

Agents and users can inspect the evidence behind a memory.

Memory is context, not unquestionable truth.

### 10.5 Update

Memory evolves.

New observations may:

* Confirm older memory
* Supersede older memory
* Contradict older memory
* Mark older memory obsolete
* Increase or decrease confidence
* Extend or shorten validity
* Link to additional evidence

## 11. Memory Scopes

The system supports four memory scopes.

```text
session
project
user
organization
```

## 11.1 Session Memory

Session memory is temporary context used during the current conversation or agent run.

Examples:

* Current task state
* Recent user instructions
* Working assumptions
* Temporary hypotheses
* Recently inspected files
* Intermediate tool results
* Draft plans
* Active TODOs

Session memory may be discarded after consolidation.

Session memory must not automatically become durable memory.

## 11.2 Project Memory

Project memory is durable memory scoped to a repo, workspace, customer, product, service, or project.

Examples:

* The project uses PostgreSQL.
* The backend tests run with `pnpm test:api`.
* The auth middleware lives in `apps/api/src/auth`.
* Do not modify generated Prisma files.
* The migration strategy using advisory locks was chosen after Redis locks failed under long-running jobs.
* Full integration tests require Docker Compose.
* The billing service depends on Stripe webhook idempotency.

Project memory is the most important scope for agentic coding.

## 11.3 User Memory

User memory stores stable user preferences and durable workflow preferences that apply across projects.

Examples:

* User prefers concise implementation-first answers.
* User prefers conventional commit messages.
* User prefers diagrams for architecture explanations.
* User prefers TypeScript examples unless another language is specified.
* User prefers direct recommendations over broad option lists.

User memory must be conservative. It should be written only when a preference is explicit, repeated, or confirmed.

## 11.4 Organization Memory

Organization memory stores shared conventions, policies, and constraints.

Examples:

* Pull requests require security review for authentication changes.
* Production deploys require two approvals.
* Internal services use OpenTelemetry.
* Architecture decisions require ADRs.
* Secrets must be managed through the approved secret manager.

Organization memory should require stricter write controls than project memory.

## 12. Core Memory Unit: Observation

The primary durable memory unit is an `Observation`.

An observation is a structured, source-backed claim about something useful for future work.

An observation is not:

* A raw transcript
* A raw event
* A hidden reasoning trace
* An ungrounded summary
* A generic note with no future utility

## 12.1 Observation Rust Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: Uuid,

    pub scope: MemoryScope,
    pub project_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub session_id: String,

    pub kind: MemoryKind,
    pub summary: String,

    pub entities: Vec<String>,
    pub files: Vec<String>,
    pub commands: Vec<String>,
    pub links: Vec<String>,

    pub confidence: MemoryConfidence,
    pub sensitivity: MemorySensitivity,
    pub status: MemoryStatus,

    pub evidence: Vec<EvidenceRef>,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub last_accessed_at: Option<OffsetDateTime>,
    pub last_confirmed_at: Option<OffsetDateTime>,
    pub valid_until: Option<OffsetDateTime>,

    pub supersedes: Vec<Uuid>,
    pub superseded_by: Option<Uuid>,

    pub metadata: serde_json::Value,
}
```

## 12.2 Memory Scope Enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "memory_scope", rename_all = "snake_case")]
pub enum MemoryScope {
    Session,
    Project,
    User,
    Organization,
}
```

## 12.3 Memory Kind Enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "memory_kind", rename_all = "snake_case")]
pub enum MemoryKind {
    Decision,
    Fact,
    Constraint,
    Preference,
    Procedure,
    ImplementationDetail,
    Bug,
    Fix,
    FailedAttempt,
    Todo,
    OpenQuestion,
    Dependency,
    Risk,
    Policy,
    ExternalReference,
}
```

## 12.4 Memory Confidence Enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "memory_confidence", rename_all = "snake_case")]
pub enum MemoryConfidence {
    Low,
    Medium,
    High,
}
```

## 12.5 Memory Sensitivity Enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "memory_sensitivity", rename_all = "snake_case")]
pub enum MemorySensitivity {
    Public,
    Internal,
    Private,
    Secret,
}
```

## 12.6 Memory Status Enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "memory_status", rename_all = "snake_case")]
pub enum MemoryStatus {
    Active,
    Unconfirmed,
    Superseded,
    Obsolete,
    Conflicted,
    Deleted,
}
```

## 13. Evidence

Every durable memory must have provenance.

## 13.1 Evidence Rust Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub id: Uuid,
    pub observation_id: Uuid,

    pub source_type: EvidenceSourceType,
    pub source_id: String,
    pub source_location: Option<String>,
    pub excerpt: Option<String>,

    pub created_at: OffsetDateTime,
}
```

## 13.2 Evidence Source Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceSourceType {
    Message,
    ToolCall,
    File,
    Terminal,
    Commit,
    Issue,
    PullRequest,
    Document,
    Web,
    ManualEntry,
}
```

Every durable memory must have at least one evidence reference unless it is manually entered or explicitly confirmed by the user.

## 14. Observation Kinds

### 14.1 Decision

A durable choice made by the user, team, project, or agent with approval.

Example:

```text
Decision: Use PostgreSQL advisory locks for job deduplication.
Rationale: Redis locks expired too early during long-running jobs.
```

Decision memories should include rationale when available.

### 14.2 Fact

A stable factual claim about the project, user, organization, or environment.

Example:

```text
Fact: The backend service uses FastAPI and PostgreSQL.
```

Facts must be source-backed and scoped.

### 14.3 Constraint

A limitation, rule, compatibility requirement, or non-negotiable boundary.

Example:

```text
Constraint: The deployment target only supports Node.js 20.
```

Constraints must be retrieved before planning or implementation.

### 14.4 Preference

A durable preference expressed or confirmed by the user or team.

Example:

```text
Preference: User prefers short PR summaries with bullets and no marketing language.
```

Preferences must not be inferred from one weak signal.

### 14.5 Procedure

A repeatable method for doing something.

Example:

```text
Procedure: Run backend tests with `pnpm test:api`.
```

Procedures should include commands, expected working directory, prerequisites, and caveats when available.

### 14.6 Implementation Detail

A useful technical detail about the system.

Example:

```text
Implementation detail: Webhook idempotency is handled in `billing/webhooks.ts`.
```

Implementation details should link to files when possible.

### 14.7 Bug

A known issue, recurring failure mode, or previously diagnosed defect.

Example:

```text
Bug: Integration tests hang if Docker Compose is not running.
```

Bug memories should include symptoms, root cause, affected files, and fix status when available.

### 14.8 Fix

A resolved issue or known remedy.

Example:

```text
Fix: Queue duplication was resolved by setting worker concurrency to 1 during migration jobs.
```

Fix memories should link to related bug memories when possible.

### 14.9 Failed Attempt

A non-trivial approach that was tried and found unsuitable.

Example:

```text
Failed attempt: Replacing the queue with BullMQ caused duplicate execution because worker concurrency was misconfigured.
```

Failed attempts are valuable because they prevent repeated wasted effort.

### 14.10 TODO

An unresolved task that should be remembered across sessions.

Example:

```text
TODO: Add retry coverage for webhook idempotency tests.
```

TODO memories should include owner, scope, and status when available.

### 14.11 Open Question

A known unresolved question.

Example:

```text
Open question: Whether billing retries should be handled by the worker or by Stripe retry configuration remains undecided.
```

Open questions must not be treated as decisions.

### 14.12 Dependency

A library, service, API, platform, or external system relevant to the project.

Example:

```text
Dependency: The billing service depends on Stripe webhook signatures.
```

### 14.13 Risk

A known risk, fragile area, security concern, or operational hazard.

Example:

```text
Risk: Raw terminal logs may contain secrets and must not be stored without filtering.
```

### 14.14 Policy

A rule or governance requirement.

Example:

```text
Policy: Authentication changes require security review before merge.
```

## 15. PostgreSQL and pgvector

PostgreSQL is the canonical store.

pgvector is used for semantic recall.

pgvector must not be treated as the canonical memory system. Embeddings are retrieval indexes over structured observations.

## 15.1 Required Extensions

```sql
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
```

## 15.2 Enum Types

```sql
CREATE TYPE memory_scope AS ENUM (
  'session',
  'project',
  'user',
  'organization'
);

CREATE TYPE memory_kind AS ENUM (
  'decision',
  'fact',
  'constraint',
  'preference',
  'procedure',
  'implementation_detail',
  'bug',
  'fix',
  'failed_attempt',
  'todo',
  'open_question',
  'dependency',
  'risk',
  'policy',
  'external_reference'
);

CREATE TYPE memory_confidence AS ENUM (
  'low',
  'medium',
  'high'
);

CREATE TYPE memory_sensitivity AS ENUM (
  'public',
  'internal',
  'private',
  'secret'
);

CREATE TYPE memory_status AS ENUM (
  'active',
  'unconfirmed',
  'superseded',
  'obsolete',
  'conflicted',
  'deleted'
);
```

## 15.3 Observations Table

```sql
CREATE TABLE observations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  scope memory_scope NOT NULL,
  project_id UUID,
  user_id UUID,
  organization_id UUID,
  session_id TEXT NOT NULL,

  kind memory_kind NOT NULL,
  summary TEXT NOT NULL,

  confidence memory_confidence NOT NULL DEFAULT 'medium',
  sensitivity memory_sensitivity NOT NULL DEFAULT 'internal',
  status memory_status NOT NULL DEFAULT 'active',

  valid_until TIMESTAMPTZ,
  last_accessed_at TIMESTAMPTZ,
  last_confirmed_at TIMESTAMPTZ,

  superseded_by UUID REFERENCES observations(id),
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

  search_tsv tsvector GENERATED ALWAYS AS (
    setweight(to_tsvector('english', coalesce(summary, '')), 'A') ||
    setweight(to_tsvector('english', coalesce(metadata::text, '')), 'C')
  ) STORED,

  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

## 15.4 Evidence Table

```sql
CREATE TABLE evidence (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,

  source_type TEXT NOT NULL,
  source_id TEXT NOT NULL,
  source_location TEXT,
  excerpt TEXT,

  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX evidence_observation_id_idx
ON evidence (observation_id);
```

## 15.5 File Links

```sql
CREATE TABLE observation_files (
  observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  file_path TEXT NOT NULL,

  PRIMARY KEY (observation_id, file_path)
);

CREATE INDEX observation_files_file_path_idx
ON observation_files (file_path);
```

## 15.6 Entity Links

```sql
CREATE TABLE observation_entities (
  observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  entity TEXT NOT NULL,

  PRIMARY KEY (observation_id, entity)
);

CREATE INDEX observation_entities_entity_idx
ON observation_entities (entity);
```

## 15.7 Commands

```sql
CREATE TABLE observation_commands (
  observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  command TEXT NOT NULL,

  PRIMARY KEY (observation_id, command)
);
```

## 15.8 Supersession Links

```sql
CREATE TABLE observation_supersessions (
  newer_observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  older_observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  reason TEXT,

  PRIMARY KEY (newer_observation_id, older_observation_id)
);
```

## 15.9 Conflicts

```sql
CREATE TABLE observation_conflicts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  left_observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  right_observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,

  conflict_type TEXT NOT NULL,
  description TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'open',

  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  resolved_at TIMESTAMPTZ
);
```

## 15.10 Embeddings

Embeddings must be stored separately from observations so the system can support multiple embedding models.

```sql
CREATE TABLE observation_embeddings (
  observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,

  model TEXT NOT NULL,
  dimensions INTEGER NOT NULL,
  embedding vector(1536) NOT NULL,

  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  PRIMARY KEY (observation_id, model)
);
```

The vector dimension must match the selected embedding model.

Examples:

```sql
embedding vector(1536)
embedding vector(1024)
embedding vector(768)
```

## 15.11 Event Buffer

Raw events are stored only temporarily and must be filtered before persistence.

```sql
CREATE TABLE session_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  session_id TEXT NOT NULL,
  project_id UUID,
  user_id UUID,
  organization_id UUID,

  event_type TEXT NOT NULL,
  payload JSONB NOT NULL DEFAULT '{}'::jsonb,

  sensitivity memory_sensitivity NOT NULL DEFAULT 'internal',
  redacted BOOLEAN NOT NULL DEFAULT false,

  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Retention for `session_events` must be configurable and short by default.

## 15.12 Audit Log

```sql
CREATE TABLE memory_audit_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  actor_type TEXT NOT NULL,
  actor_id TEXT,

  action TEXT NOT NULL,
  observation_id UUID REFERENCES observations(id) ON DELETE SET NULL,

  before JSONB,
  after JSONB,

  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

## 15.13 Indexes

```sql
CREATE INDEX observations_scope_project_idx
ON observations (scope, project_id, status, kind);

CREATE INDEX observations_user_idx
ON observations (user_id, status);

CREATE INDEX observations_org_idx
ON observations (organization_id, status);

CREATE INDEX observations_kind_idx
ON observations (kind);

CREATE INDEX observations_created_at_idx
ON observations (created_at DESC);

CREATE INDEX observations_status_idx
ON observations (status);

CREATE INDEX observations_search_tsv_idx
ON observations USING GIN (search_tsv);

CREATE INDEX observations_metadata_gin_idx
ON observations USING GIN (metadata);

CREATE INDEX observation_embeddings_hnsw_idx
ON observation_embeddings
USING hnsw (embedding vector_cosine_ops);
```

## 16. Hybrid Retrieval

The system must use hybrid retrieval by default.

Hybrid retrieval combines:

* Scope filter
* Project filter
* User filter
* Organization filter
* Status filter
* Sensitivity filter
* Kind filter
* File filter
* Entity filter
* PostgreSQL full-text search
* pgvector semantic similarity
* Confidence score
* Recency score
* Evidence score
* Conflict penalty
* Supersession penalty

## 16.1 Scoring Model

Recommended conceptual scoring:

```text
final_score =
  semantic_score * 0.45
  + keyword_score * 0.30
  + confidence_score * 0.10
  + recency_score * 0.05
  + evidence_score * 0.05
  + file_or_entity_match_score * 0.05
  - conflict_penalty
  - obsolete_penalty
```

The exact weights may be configurable.

## 16.2 Hybrid Query Pattern

```sql
WITH vector_matches AS (
  SELECT
    o.id,
    1 - (e.embedding <=> $1::vector) AS vector_score
  FROM observation_embeddings e
  JOIN observations o ON o.id = e.observation_id
  WHERE
    o.status = 'active'
    AND o.scope = $2
    AND ($3::uuid IS NULL OR o.project_id = $3)
    AND o.sensitivity != 'secret'
  ORDER BY e.embedding <=> $1::vector
  LIMIT 50
),

text_matches AS (
  SELECT
    o.id,
    ts_rank_cd(o.search_tsv, plainto_tsquery('english', $4)) AS text_score
  FROM observations o
  WHERE
    o.status = 'active'
    AND o.scope = $2
    AND ($3::uuid IS NULL OR o.project_id = $3)
    AND o.sensitivity != 'secret'
    AND o.search_tsv @@ plainto_tsquery('english', $4)
  ORDER BY text_score DESC
  LIMIT 50
),

combined AS (
  SELECT
    o.id,
    o.kind,
    o.summary,
    o.confidence,
    o.status,
    o.created_at,
    COALESCE(vm.vector_score, 0) AS vector_score,
    COALESCE(tm.text_score, 0) AS text_score,
    CASE o.confidence
      WHEN 'high' THEN 1.0
      WHEN 'medium' THEN 0.7
      WHEN 'low' THEN 0.4
    END AS confidence_score,
    LEAST(
      1.0,
      1.0 / (
        1.0 + EXTRACT(EPOCH FROM (now() - o.created_at)) / 2592000.0
      )
    ) AS recency_score
  FROM observations o
  LEFT JOIN vector_matches vm ON vm.id = o.id
  LEFT JOIN text_matches tm ON tm.id = o.id
  WHERE vm.id IS NOT NULL OR tm.id IS NOT NULL
)

SELECT
  *,
  (
    vector_score * 0.45 +
    text_score * 0.30 +
    confidence_score * 0.15 +
    recency_score * 0.10
  ) AS final_score
FROM combined
ORDER BY final_score DESC
LIMIT $5;
```

## 17. Recall Policy

Retrieval must be task-aware.

## 17.1 New Session Recall

At session start, retrieve:

```text
Project summary
Recent unresolved TODOs
Active constraints
Relevant decisions
Known risks
User preferences
Recently confirmed procedures
```

Do not inject all project memory.

## 17.2 File Edit Recall

Before editing a file, retrieve:

```text
Memories linked to the file
Relevant implementation details
Known bugs involving the file
Failed attempts involving the file
Procedures involving the file
Related decisions
```

## 17.3 Debugging Recall

For debugging, retrieve:

```text
Prior bugs
Prior fixes
Failed attempts
Known pitfalls
Test commands
Environment constraints
Recent changes
```

## 17.4 Architecture Recall

For architectural work, retrieve:

```text
Prior decisions
Rationale
Constraints
Risks
Policies
Dependencies
Open questions
Superseded decisions as historical context
```

## 17.5 Preference Recall

User preferences should be retrieved only when relevant to output style, workflow, tools, or interaction model.

## 18. Context Budget

Memory injection must be bounded by token budget.

Default budget policy:

```text
Session-start memory budget: 1,000 tokens
Task recall budget: 1,200 tokens
File recall budget: 700 tokens
Architecture recall budget: 1,800 tokens
Debugging recall budget: 1,500 tokens
User preference budget: 200 tokens
```

When budget is limited, memory should be prioritized in this order:

```text
Active constraints
Relevant decisions
File-linked implementation details
Known bugs and fixes
Failed attempts
Procedures
Open TODOs
User preferences
Historical/superseded context
```

## 19. MCP Interface

The system exposes MCP tools for agents.

MCP must be the agent-facing protocol layer, not the core architecture.

## 19.1 MCP Design Rules

The MCP layer must:

* Validate inputs.
* Enforce authorization boundaries.
* Call core services.
* Return compact structured outputs.
* Avoid leaking private or secret data.
* Avoid embedding business logic in tool handlers.
* Provide stable tool schemas.
* Return provenance identifiers for material memories.
* Support task-aware recall.
* Support explicit memory search.
* Support memory verification through `memory.get`.

## 19.2 MCP Tools

### 19.2.1 `memory.recall`

Task-aware recall.

Input:

```json
{
  "task": "debug failed auth middleware tests",
  "scope": "project",
  "project_id": "uuid",
  "files": ["apps/api/src/auth/middleware.ts"],
  "token_budget": 1200
}
```

Output:

```json
{
  "memories": [
    {
      "id": "uuid",
      "kind": "failed_attempt",
      "summary": "Previous attempt to mock auth context failed because middleware reads from request extensions.",
      "confidence": "high",
      "status": "active",
      "evidence_count": 2
    }
  ]
}
```

### 19.2.2 `memory.search`

Hybrid memory search.

Input:

```json
{
  "query": "auth middleware request extensions",
  "scope": "project",
  "project_id": "uuid",
  "kinds": ["decision", "failed_attempt", "bug"],
  "files": ["apps/api/src/auth/middleware.ts"],
  "limit": 10
}
```

### 19.2.3 `memory.get`

Fetch full memory with provenance.

Input:

```json
{
  "id": "uuid"
}
```

### 19.2.4 `memory.write`

Write a source-backed observation.

Input:

```json
{
  "scope": "project",
  "project_id": "uuid",
  "session_id": "session-123",
  "kind": "decision",
  "summary": "Use PostgreSQL advisory locks for job deduplication.",
  "confidence": "high",
  "sensitivity": "internal",
  "evidence": [
    {
      "source_type": "message",
      "source_id": "msg-123",
      "excerpt": "Let's use advisory locks here."
    }
  ]
}
```

### 19.2.5 `memory.update`

Update an existing memory.

Input:

```json
{
  "id": "uuid",
  "summary": "Updated summary",
  "confidence": "high",
  "status": "active"
}
```

### 19.2.6 `memory.mark_obsolete`

Mark a memory obsolete.

Input:

```json
{
  "id": "uuid",
  "reason": "Project upgraded from Node 20 to Node 22."
}
```

### 19.2.7 `memory.consolidate_session`

Convert session events into durable observations.

Input:

```json
{
  "session_id": "session-123",
  "project_id": "uuid"
}
```

### 19.2.8 `memory.link_file`

Link an observation to a file.

Input:

```json
{
  "observation_id": "uuid",
  "file_path": "apps/api/src/auth/middleware.ts"
}
```

### 19.2.9 `memory.list_conflicts`

List unresolved memory conflicts.

Input:

```json
{
  "project_id": "uuid"
}
```

### 19.2.10 `memory.resolve_conflict`

Resolve a memory conflict.

Input:

```json
{
  "conflict_id": "uuid",
  "resolution": "left_wins",
  "reason": "The newer observation is confirmed by the latest migration."
}
```

### 19.2.11 `memory.delete`

Soft-delete a memory.

Input:

```json
{
  "id": "uuid",
  "reason": "User requested deletion."
}
```

## 20. Event Capture

The system may capture temporary raw events during a session.

Events may include:

* User prompts
* Assistant responses
* Tool calls
* Tool outputs
* File reads
* File writes
* Shell commands
* Test results
* Error messages
* Code diffs
* Commit metadata
* Issue references
* Pull request references
* Explicit memory commands
* Session start events
* Session end events

Raw events are not durable memory by default.

## 20.1 Capture Policy

| Event Type             |       Capture Temporarily | Persist Raw | Eligible for Observation |
| ---------------------- | ------------------------: | ----------: | -----------------------: |
| User prompt            |                       Yes |          No |                      Yes |
| Assistant response     |                       Yes |          No |                      Yes |
| File read              |             Metadata only |          No |                Sometimes |
| File write             | Metadata and diff summary |          No |                      Yes |
| Shell command          |                       Yes |          No |                Sometimes |
| Terminal output        |             Filtered only |          No |                Sometimes |
| Error output           |             Filtered only |          No |                      Yes |
| Test result            |              Summary only |          No |                      Yes |
| Secret-looking content |                        No |          No |                       No |
| Private block          |                        No |          No |                       No |

## 21. Privacy and Safety

Privacy is a primary system requirement.

The system must prevent sensitive data from entering durable memory.

## 21.1 Sensitivity Levels

```text
public
internal
private
secret
```

### Public

Safe to store and show broadly.

### Internal

Project or organization context that should not be public.

### Private

User-sensitive content that should only be stored with explicit permission.

### Secret

Credentials, tokens, API keys, passwords, private keys, session cookies, access tokens, or other highly sensitive material.

Secret content must not be stored.

## 21.2 Private Blocks

The system should support explicit user-controlled exclusion blocks.

```text
<private>
Do not persist this content.
</private>
```

Content inside private blocks must be excluded from:

* Event storage
* Memory extraction
* Full-text indexing
* Embedding generation
* Search results
* MCP responses
* Audit excerpts

The live agent may use private-block content during the current session, but the system must not persist it.

## 21.3 Secret Detection

The system must scan captured inputs and tool outputs for likely secrets.

Secret patterns include:

* API keys
* Access tokens
* OAuth tokens
* SSH keys
* Private keys
* Passwords
* Connection strings
* Session cookies
* JWTs
* Cloud provider credentials
* `.env` contents
* Database URLs

Detected secrets must be redacted before storage, indexing, summarization, or embedding.

## 21.4 Storage Exclusion Rules

The system must not store:

* Secrets
* Credentials
* Raw `.env` files
* Private keys
* Full terminal logs containing sensitive content
* Private-block content
* Full chat transcripts by default
* Hidden reasoning traces
* Unfiltered tool outputs
* Personal information unrelated to future task continuity

## 21.5 User Controls

Users must be able to:

* Disable memory globally
* Disable memory for a project
* Disable memory for a session
* Mark content as private
* View stored memory
* Edit stored memory
* Delete stored memory
* Export stored memory
* Confirm memory
* Mark memory obsolete
* Mark memory incorrect
* Set retention preferences

## 22. Consolidation

Consolidation converts temporary session events into durable observations.

## 22.1 Consolidation Inputs

Consolidation may use:

* Session event summaries
* Tool call summaries
* File diff summaries
* Error summaries
* Test outputs
* Explicit user instructions
* Existing memory
* Project metadata
* Agent final summaries

## 22.2 Consolidation Pipeline

```text
Raw session events
  → private block removal
  → secret detection and redaction
  → sensitivity classification
  → importance scoring
  → candidate observation extraction
  → evidence attachment
  → deduplication
  → contradiction detection
  → confidence assignment
  → lifecycle assignment
  → durable storage
  → full-text index update
  → embedding generation
```

## 22.3 Importance Scoring

High-value signals:

* Explicit user instruction
* Repeated across sessions
* Architecture relevance
* Setup or command relevance
* Bug or fix relevance
* Failed attempt relevance
* Constraint relevance
* Open task relevance
* Frequently modified file relevance
* User or team preference relevance
* Operational hazard relevance

Low-value signals:

* Temporary brainstorming
* One-off generic questions
* Obvious information derivable from files
* Raw logs
* Redundant file reads
* Low-confidence speculation
* Generic coding facts
* Unverified assumptions

## 23. Conflict Detection

The system must detect conflicts between new and existing memories.

Conflict categories:

```text
same entity + same property + incompatible value
same file + incompatible implementation assumption
same command + incompatible expected result
same dependency + different version
same user preference + different preference
same decision topic + incompatible decision
same policy area + incompatible policy
```

When a conflict is detected, the system should:

* Store the new observation as `conflicted` or `unconfirmed`.
* Create an `observation_conflicts` record.
* Avoid using the conflicted memory as authoritative.
* Surface the conflict to the user or agent.
* Allow explicit resolution.

## 24. Supersession and Decay

Old memories must not remain equally authoritative forever.

Memory can become:

```text
active
unconfirmed
superseded
obsolete
conflicted
deleted
```

A newer memory may supersede one or more older memories.

Example:

```text
Old memory:
Project uses Node 20.

New memory:
Project upgraded to Node 22.

Result:
Old memory is superseded.
New memory is active.
```

Superseded memory should not be deleted immediately. It may be useful as historical context.

## 25. Documentation Freshness with Context7

The development workflow must use Context7 whenever implementation depends on current library APIs, SDK behavior, framework conventions, or version-specific examples.

Context7 is used to retrieve up-to-date, version-specific documentation and code examples for libraries directly into AI coding workflows.

## 25.1 Required Context7 Usage

Use Context7 when working with:

* MCP SDKs
* Rust MCP server libraries
* `sqlx`
* `pgvector`
* `axum`
* `tokio`
* `clap`
* `serde`
* `tracing`
* `testcontainers`
* `cargo audit`
* `cargo deny`
* PostgreSQL vector indexing patterns
* Any dependency whose API may have changed
* Any library introduced or upgraded recently

## 25.2 Agent Instruction

When asking an AI coding assistant to implement or modify code, include:

```text
use context7
```

Example:

```text
Implement a Rust MCP tool handler using the current MCP SDK patterns. use context7
```

Example:

```text
Create sqlx migrations for PostgreSQL with pgvector HNSW indexes. use context7
```

Example:

```text
Implement an axum route for memory search with typed request validation. use context7
```

## 25.3 Context7 Policy

The agent must not rely only on model memory for library APIs when Context7 is available.

The agent must use Context7 before:

* Adding new dependencies.
* Using unfamiliar APIs.
* Implementing MCP protocol behavior.
* Writing pgvector-specific SQL.
* Writing `sqlx` compile-time checked queries.
* Writing framework integration code.
* Upgrading dependencies.
* Debugging version-specific API errors.

## 25.4 Documentation Provenance

When Context7 influences implementation, the related development note or PR description should mention the library and topic consulted.

Example:

```text
Used Context7 for current sqlx migration and query macro documentation.
```

The system does not need to store Context7 docs as memory unless the user explicitly wants durable project memory about a library decision or implementation detail.

## 26. Linting, Formatting, and Static Checks

The project must enforce linting and formatting.

No change should be considered complete unless it passes the required checks.

## 26.1 Required Local Checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-features
cargo audit
cargo deny check
```

## 26.2 SQL Checks

If using `sqlx` compile-time query checking, the project must support:

```bash
cargo sqlx prepare --workspace -- --all-features
```

or an equivalent offline query verification workflow.

Migrations must be tested against PostgreSQL with pgvector enabled.

## 26.3 Formatting

Rust code must be formatted with `rustfmt`.

SQL migrations should follow a consistent formatting style.

JSON, TOML, Markdown, and YAML files should be formatted through appropriate tooling where available.

## 26.4 Clippy Policy

Clippy warnings are treated as errors.

Required:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

No `#[allow(...)]` should be added unless justified in code comments.

## 26.5 Security and Dependency Checks

Required:

```bash
cargo audit
cargo deny check
```

The project should define `deny.toml` for:

* Duplicate dependencies
* Vulnerable dependencies
* Banned licenses
* Unknown licenses
* Unmaintained crates when relevant
* Multiple versions of important crates where avoidable

## 26.6 Test Policy

The project must include:

* Unit tests for core memory lifecycle logic.
* Unit tests for privacy filtering.
* Unit tests for conflict detection.
* Unit tests for ranking.
* Integration tests for PostgreSQL persistence.
* Integration tests for pgvector search.
* Integration tests for MCP tool calls.
* Migration tests.
* Regression tests for secret redaction.

## 26.7 CI Requirements

Continuous integration must run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
cargo deny check
```

CI must provision PostgreSQL with pgvector for integration tests.

## 27. Development Quality Rules

The system must follow these engineering rules:

* No business logic in MCP handlers.
* No direct SQL in MCP handlers.
* No raw secrets in logs.
* No panic-based control flow in production paths.
* No unchecked persistence of tool outputs.
* No memory writes without sensitivity classification.
* No durable memory without provenance.
* No vector search without structured filters.
* No automatic use of conflicted memory as authoritative.
* No hidden deletion; deletion must be soft-deleted or audited unless legally required otherwise.
* No undocumented schema changes.
* No migration without tests.
* No new dependency without checking current documentation and license posture.
* No implementation using uncertain library APIs without Context7.

## 28. Error Handling

The system should define domain-specific errors.

```rust
#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    #[error("observation not found: {0}")]
    ObservationNotFound(Uuid),

    #[error("invalid memory scope")]
    InvalidScope,

    #[error("secret content cannot be persisted")]
    SecretContentRejected,

    #[error("conflicting memory detected")]
    ConflictDetected,

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("embedding provider error: {0}")]
    EmbeddingProvider(String),

    #[error("consolidation provider error: {0}")]
    ConsolidationProvider(String),

    #[error("authorization denied")]
    AuthorizationDenied,
}
```

Errors exposed through MCP must be safe and must not leak secrets, raw SQL, connection strings, or private data.

## 29. Logging and Observability

Use structured logging with `tracing`.

Logs should include:

* Request IDs
* Session IDs
* Project IDs where safe
* Tool names
* Latency
* Result counts
* Error classes
* Worker job IDs

Logs must not include:

* Secrets
* Raw private content
* Full prompts by default
* Full terminal output
* Full memory contents unless debug logging is explicitly enabled and safe

Metrics should track:

* Memory writes
* Memory searches
* Recall latency
* Embedding latency
* Consolidation latency
* Conflict count
* Redaction count
* Secret rejection count
* Search result count
* MCP tool error count

## 30. Review UI

The system should provide a local review interface.

The UI should support:

* Search memories
* Filter by scope
* Filter by project
* Filter by kind
* Filter by status
* Filter by sensitivity
* Filter by file
* View evidence
* View conflicts
* Edit summary
* Confirm memory
* Mark obsolete
* Resolve conflicts
* Delete memory
* Export memory
* Show recently created memories
* Show low-confidence memories
* Show memories without embeddings
* Show stale memories

The review UI must never display secret content because secret content must not be stored.

## 31. Authorization

The system must enforce scope boundaries.

Rules:

* Project memory is available only within that project context.
* User memory is available only to the owning user.
* Organization memory is available only according to organization policy.
* Private memory requires explicit permission.
* Secret memory must not exist.
* MCP calls must be validated against configured access context.

## 32. Configuration

The system should support configuration through file and environment variables.

Example config:

```toml
[database]
url = "postgres://..."

[memory]
default_scope = "project"
event_retention_days = 7
max_recall_tokens = 1200
allow_user_memory = true
allow_org_memory = false

[privacy]
enable_private_blocks = true
enable_secret_scanner = true
reject_secret_writes = true

[embedding]
provider = "openai"
model = "text-embedding-3-small"
dimensions = 1536

[mcp]
transport = "stdio"

[lint]
deny_warnings = true
```

Environment variables may override config file values.

## 33. Embedding Policy

Embeddings should be generated from compact, safe memory text.

Embedding input should include:

```text
kind
summary
entities
files
commands
selected metadata
```

Embedding input must not include:

* Secrets
* Private-block content
* Raw terminal logs
* Full transcripts
* Large code blocks
* Hidden reasoning traces

Embedding regeneration is required when:

* Summary changes
* Kind changes
* Entities change
* File links change
* Embedding model changes
* Embedding dimensions change

## 34. Memory Write Policy

Default rule:

```text
Capture broadly, store selectively.
```

Store when the information is:

```text
stable
useful
safe
source-backed
future-relevant
properly scoped
```

Do not store when the information is:

```text
temporary
sensitive
secret
unverified
generic
obvious
duplicative
not future-useful
```

## 35. Memory Deletion

Deletion should be soft by default:

```text
status = deleted
```

Hard deletion may be required for privacy or compliance requests.

Deletion must remove or invalidate:

* Observation row
* Evidence
* File links
* Entity links
* Command links
* Embeddings
* Conflict links
* Search visibility

Audit records may remain only if they do not contain private or secret content.

## 36. Agent Behavior Requirements

Agents using the memory system should:

* Call `memory.recall` at the start of non-trivial project work.
* Call `memory.recall` before modifying important files.
* Call `memory.search` when prior context may exist.
* Call `memory.get` before relying on critical memories.
* Write memory only when the observation is durable, useful, and source-backed.
* Avoid writing temporary hypotheses as high-confidence memory.
* Mark superseded or obsolete memory when new facts replace old facts.
* Surface conflicts instead of silently choosing one.
* Use Context7 when implementation requires current library documentation.
* Run linting and tests before claiming implementation completion.

## 37. Example Agent Workflow

### 37.1 Start Work

```text
Agent receives task:
"Continue fixing the billing webhook idempotency bug."

Agent calls:
memory.recall({
  task: "Continue fixing the billing webhook idempotency bug",
  scope: "project",
  project_id: "...",
  token_budget: 1200
})
```

### 37.2 Recall Result

```text
Relevant memories:
- Decision: Webhook idempotency is handled at the worker layer.
- Bug: Duplicate webhook processing occurred when retry jobs overlapped.
- Failed attempt: Redis lock expiry caused double processing.
- Procedure: Run billing tests with `pnpm test:billing`.
```

### 37.3 Implementation

Agent uses recalled memory to avoid repeating failed approaches.

If using Rust libraries or MCP APIs, the implementation prompt includes:

```text
use context7
```

### 37.4 Completion

At the end of the session, the system consolidates:

```text
Fix: Duplicate webhook processing was resolved by using PostgreSQL advisory locks with transaction-scoped lock release.
Evidence: session, files, test output.
```

## 38. Acceptance Criteria

The system is acceptable when:

* Memory is stored as structured observations.
* Durable memory requires provenance.
* PostgreSQL is the canonical store.
* pgvector is used for semantic recall.
* Full-text search is supported.
* Hybrid retrieval is implemented.
* MCP tools expose recall, search, get, write, update, obsolete, delete, consolidate, and conflict resolution.
* Secret content is rejected.
* Private blocks are excluded from persistence.
* Conflicted memory is not treated as authoritative.
* Superseded memory is preserved as historical context.
* Context injection is bounded by token budget.
* Context7 is required for current library documentation during implementation.
* Linting is mandatory.
* Formatting is mandatory.
* Tests are mandatory.
* Security and dependency checks are mandatory.
* CI provisions PostgreSQL with pgvector.
* The memory core is independent from MCP.
* The system can run as a Rust binary.
* Users can inspect, edit, obsolete, delete, and export memory.

## 39. Final Design Summary

The system is a Rust-based cross-session agent memory daemon.

It uses PostgreSQL as the source of truth and pgvector as the semantic recall layer.

It exposes MCP tools to AI agents while keeping the memory core independent of MCP.

It stores observations, not transcripts.

It retrieves memory selectively through hybrid search.

It treats privacy, provenance, conflict detection, and lifecycle management as core system concerns.

It requires Context7 for up-to-date implementation documentation and requires linting, formatting, tests, and static checks for development quality.

The defining rule remains:

> Persistent memory should preserve continuity, not preserve history.

