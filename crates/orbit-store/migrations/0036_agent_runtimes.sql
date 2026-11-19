CREATE TABLE IF NOT EXISTS agent_runtimes (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    agent_id uuid NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    status text NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'stopping', 'stopped', 'failed')),
    task text NOT NULL,
    harness_kind text NOT NULL,
    last_execution_id uuid REFERENCES executions(id) ON DELETE SET NULL,
    last_error text,
    next_run_at timestamptz NOT NULL DEFAULT now(),
    created_by uuid REFERENCES users(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    stopped_at timestamptz
);
CREATE UNIQUE INDEX IF NOT EXISTS agent_runtimes_running_idx
    ON agent_runtimes (org_id, agent_id)
    WHERE status IN ('running', 'stopping');
CREATE INDEX IF NOT EXISTS agent_runtimes_status_idx ON agent_runtimes (status);
