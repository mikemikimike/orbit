-- Per-call tool policy rules. Org-level rules have agent_id NULL; agent rules override
-- and extend them. Rules can only restrict the platform decision (deny > approval >
-- allow); they never grant capabilities.
CREATE TABLE IF NOT EXISTS tool_policies (
    id         uuid PRIMARY KEY,
    org_id     uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    agent_id   uuid REFERENCES agents(id) ON DELETE CASCADE,
    name       text NOT NULL,
    priority   integer NOT NULL DEFAULT 100,
    enabled    boolean NOT NULL DEFAULT true,
    action     text NOT NULL CHECK (action IN ('allow', 'deny', 'require_approval')),
    matcher    jsonb NOT NULL DEFAULT '{}'::jsonb,
    reason     text,
    impact     text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE NULLS NOT DISTINCT (org_id, agent_id, name)
);

CREATE INDEX IF NOT EXISTS tool_policies_org_idx
    ON tool_policies (org_id, agent_id, priority);
