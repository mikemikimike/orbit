-- Outputs can target an Agent Team. Deliveries now reference either an execution or a
-- terminal team run.
ALTER TABLE outputs
    ADD COLUMN IF NOT EXISTS team_id uuid REFERENCES agent_teams(id) ON DELETE CASCADE;

ALTER TABLE outputs DROP CONSTRAINT IF EXISTS outputs_scope_check;
ALTER TABLE outputs
    ADD CONSTRAINT outputs_scope_check CHECK (agent_id IS NULL OR team_id IS NULL);

CREATE INDEX IF NOT EXISTS outputs_team_idx ON outputs (org_id, team_id);

ALTER TABLE output_deliveries ALTER COLUMN execution_id DROP NOT NULL;
ALTER TABLE output_deliveries
    ADD COLUMN IF NOT EXISTS team_run_id uuid REFERENCES team_runs(id) ON DELETE CASCADE;

CREATE UNIQUE INDEX IF NOT EXISTS output_deliveries_team_unique
    ON output_deliveries (output_id, team_run_id)
    WHERE team_run_id IS NOT NULL;

ALTER TABLE output_deliveries DROP CONSTRAINT IF EXISTS output_deliveries_target_check;
ALTER TABLE output_deliveries
    ADD CONSTRAINT output_deliveries_target_check
    CHECK ((execution_id IS NOT NULL)::int + (team_run_id IS NOT NULL)::int = 1);
