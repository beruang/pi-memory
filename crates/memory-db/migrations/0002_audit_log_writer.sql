-- Add performance index on memory_audit_log for fast observation-scoped lookups.
-- The memory_audit_log table itself is already defined in 0001_initial_schema.sql.
CREATE INDEX IF NOT EXISTS memory_audit_log_observation_created_idx
ON memory_audit_log (observation_id, created_at DESC);

CREATE INDEX IF NOT EXISTS memory_audit_log_created_at_idx
ON memory_audit_log (created_at DESC);