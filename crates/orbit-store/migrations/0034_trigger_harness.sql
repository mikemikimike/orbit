-- Harness is selected per execution/trigger instead of on the agent.
-- Nullable: legacy triggers resolve through the organization default until backfilled.
ALTER TABLE triggers ADD COLUMN IF NOT EXISTS harness_kind text;
