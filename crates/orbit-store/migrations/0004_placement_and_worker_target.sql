-- Explicit execution placement and worker target classes.
ALTER TABLE executions
    ADD COLUMN IF NOT EXISTS placement text NOT NULL DEFAULT 'auto'
    CHECK (placement IN ('auto', 'local', 'docker', 'kubernetes'));

ALTER TABLE worker_heartbeats
    ADD COLUMN IF NOT EXISTS target text NOT NULL DEFAULT 'local';

CREATE INDEX IF NOT EXISTS executions_placement_idx
    ON executions (placement, status);
