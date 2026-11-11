-- Execution-level skills and sequential agent teams.
ALTER TABLE executions ADD COLUMN IF NOT EXISTS skills text[] NOT NULL DEFAULT '{}';

CREATE TABLE IF NOT EXISTS agent_teams (
    id          uuid PRIMARY KEY,
    org_id      uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        text NOT NULL,
    description text NOT NULL DEFAULT '',
    definition  jsonb NOT NULL,
    created_by  uuid REFERENCES users(id) ON DELETE SET NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS agent_teams_org_idx ON agent_teams (org_id, created_at DESC);

CREATE TABLE IF NOT EXISTS team_runs (
    id           uuid PRIMARY KEY,
    org_id       uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    team_id      uuid NOT NULL REFERENCES agent_teams(id) ON DELETE CASCADE,
    status       text NOT NULL DEFAULT 'running',
    input        jsonb NOT NULL DEFAULT '{}'::jsonb,
    output       jsonb,
    error        text,
    requested_by uuid REFERENCES users(id) ON DELETE SET NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    finished_at  timestamptz
);

CREATE INDEX IF NOT EXISTS team_runs_org_idx ON team_runs (org_id, created_at DESC);

CREATE TABLE IF NOT EXISTS team_steps (
    id               uuid PRIMARY KEY,
    run_id           uuid NOT NULL REFERENCES team_runs(id) ON DELETE CASCADE,
    org_id           uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    step_index       integer NOT NULL,
    name             text NOT NULL,
    agent_id         uuid NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    agent_version_id uuid NOT NULL REFERENCES agent_versions(id) ON DELETE RESTRICT,
    skills           text[] NOT NULL DEFAULT '{}',
    status           text NOT NULL DEFAULT 'pending',
    execution_id     uuid REFERENCES executions(id) ON DELETE SET NULL,
    input            jsonb NOT NULL DEFAULT '{}'::jsonb,
    output           jsonb,
    error            text,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    started_at       timestamptz,
    finished_at      timestamptz,
    UNIQUE (run_id, step_index)
);

CREATE INDEX IF NOT EXISTS team_steps_run_idx ON team_steps (run_id, step_index);
