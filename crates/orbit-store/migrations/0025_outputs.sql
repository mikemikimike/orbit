-- Execution result outputs: Slack/Mattermost incoming webhooks or generic webhooks.
CREATE TABLE IF NOT EXISTS outputs (
    id          uuid PRIMARY KEY,
    org_id      uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        text NOT NULL,
    kind        text NOT NULL CHECK (kind IN ('slack', 'mattermost', 'webhook')),
    agent_id    uuid REFERENCES agents(id) ON DELETE CASCADE,
    events      text[] NOT NULL DEFAULT '{succeeded,failed,cancelled}',
    secret_ref  text NOT NULL,
    template    text,
    enabled     boolean NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, name)
);

CREATE TABLE IF NOT EXISTS output_deliveries (
    id              uuid PRIMARY KEY,
    org_id          uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    output_id       uuid NOT NULL REFERENCES outputs(id) ON DELETE CASCADE,
    execution_id    uuid NOT NULL REFERENCES executions(id) ON DELETE CASCADE,
    status          text NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'sending', 'succeeded', 'failed', 'dead')),
    attempts        integer NOT NULL DEFAULT 0,
    next_attempt_at timestamptz NOT NULL DEFAULT now(),
    last_error      text,
    response_status integer,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE (output_id, execution_id)
);

CREATE INDEX IF NOT EXISTS outputs_org_idx ON outputs (org_id, agent_id);
CREATE INDEX IF NOT EXISTS output_deliveries_due_idx
    ON output_deliveries (status, next_attempt_at);
