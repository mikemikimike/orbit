-- Agent memory: short-term and summary memory for persistent agents.
CREATE TABLE IF NOT EXISTS agent_memory (
    id          uuid PRIMARY KEY,
    org_id      uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    agent_id    uuid NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    kind        text NOT NULL CHECK (kind IN ('short_term', 'summary')),
    content     text NOT NULL,
    metadata    jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at  timestamptz NOT NULL DEFAULT now(),
    expires_at  timestamptz
);

CREATE INDEX IF NOT EXISTS agent_memory_agent_idx
    ON agent_memory (org_id, agent_id, kind, created_at DESC);
CREATE INDEX IF NOT EXISTS agent_memory_expires_idx
    ON agent_memory (expires_at) WHERE expires_at IS NOT NULL;
