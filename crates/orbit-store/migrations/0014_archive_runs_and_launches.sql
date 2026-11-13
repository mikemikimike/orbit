-- Retention bookkeeping: terminal runs and launches can be archived for a cleaner
-- default view without deleting their history.
ALTER TABLE executions
    ADD COLUMN IF NOT EXISTS archived_at timestamptz;

ALTER TABLE team_runs
    ADD COLUMN IF NOT EXISTS archived_at timestamptz;

ALTER TABLE worker_launches
    ADD COLUMN IF NOT EXISTS archived_at timestamptz;

CREATE INDEX IF NOT EXISTS executions_org_archived_idx
    ON executions (org_id, archived_at, created_at DESC);

CREATE INDEX IF NOT EXISTS team_runs_org_archived_idx
    ON team_runs (org_id, archived_at, created_at DESC);

CREATE INDEX IF NOT EXISTS worker_launches_org_archived_idx
    ON worker_launches (org_id, archived_at, created_at DESC);
