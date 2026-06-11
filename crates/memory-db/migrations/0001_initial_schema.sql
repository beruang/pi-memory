CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

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

CREATE TABLE observation_files (
  observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  file_path TEXT NOT NULL,

  PRIMARY KEY (observation_id, file_path)
);

CREATE INDEX observation_files_file_path_idx
ON observation_files (file_path);

CREATE TABLE observation_entities (
  observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  entity TEXT NOT NULL,

  PRIMARY KEY (observation_id, entity)
);

CREATE INDEX observation_entities_entity_idx
ON observation_entities (entity);

CREATE TABLE observation_commands (
  observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  command TEXT NOT NULL,

  PRIMARY KEY (observation_id, command)
);

CREATE TABLE observation_supersessions (
  newer_observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  older_observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
  reason TEXT,

  PRIMARY KEY (newer_observation_id, older_observation_id)
);

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

CREATE TABLE observation_embeddings (
  observation_id UUID NOT NULL REFERENCES observations(id) ON DELETE CASCADE,

  model TEXT NOT NULL,
  dimensions INTEGER NOT NULL,
  embedding vector(1536) NOT NULL,

  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  PRIMARY KEY (observation_id, model)
);

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

-- Indexes
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
