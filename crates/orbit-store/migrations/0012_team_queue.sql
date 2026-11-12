-- 0.6.1: durable Agent Team work queue (one item -> one TeamRun).
CREATE TABLE IF NOT EXISTS team_queue_items (
    id           uuid PRIMARY KEY,
    org_id       uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    team_id      uuid NOT NULL REFERENCES agent_teams(id) ON DELETE CASCADE,
    external_key text NOT NULL,
    repository   text,
    payload      jsonb NOT NULL DEFAULT '{}'::jsonb,
    status       text NOT NULL DEFAULT 'pending',
    team_run_id  uuid REFERENCES team_runs(id) ON DELETE SET NULL,
    error        text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (team_id, external_key)
);

CREATE INDEX IF NOT EXISTS team_queue_items_org_idx
    ON team_queue_items (org_id, team_id, status, created_at);
