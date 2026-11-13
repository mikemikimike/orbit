-- Work source polling bookkeeping and queue retry policy.
ALTER TABLE agent_teams
    ADD COLUMN IF NOT EXISTS last_poll_at timestamptz;
ALTER TABLE agent_teams
    ADD COLUMN IF NOT EXISTS last_poll_error text;

ALTER TABLE team_queue_items
    ADD COLUMN IF NOT EXISTS attempts integer NOT NULL DEFAULT 0;
ALTER TABLE team_queue_items
    ADD COLUMN IF NOT EXISTS max_attempts integer NOT NULL DEFAULT 1;
