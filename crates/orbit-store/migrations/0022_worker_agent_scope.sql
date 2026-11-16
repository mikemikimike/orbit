-- Explicit agent scope for worker profiles. Previously an empty agent_ids array meant
-- "all agents" implicitly, while a populated list was a snapshot. Make it explicit so
-- "all agents" stays dynamic and new agents are picked up automatically.
ALTER TABLE worker_profiles
    ADD COLUMN IF NOT EXISTS agent_scope text NOT NULL DEFAULT 'all'
    CHECK (agent_scope IN ('all', 'selected'));

UPDATE worker_profiles
SET agent_scope = 'selected'
WHERE cardinality(agent_ids) > 0;
