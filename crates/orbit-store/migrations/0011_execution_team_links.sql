-- Link normal executions back to their Agent Team run/step for grouped visibility.
ALTER TABLE executions
    ADD COLUMN IF NOT EXISTS team_run_id uuid REFERENCES team_runs(id) ON DELETE SET NULL;
ALTER TABLE executions
    ADD COLUMN IF NOT EXISTS team_step_index integer;

CREATE INDEX IF NOT EXISTS executions_team_run_idx
    ON executions (team_run_id, team_step_index);
