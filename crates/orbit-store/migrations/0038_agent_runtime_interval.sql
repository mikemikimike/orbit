ALTER TABLE agent_runtimes
    ADD COLUMN IF NOT EXISTS interval_seconds integer NOT NULL DEFAULT 60
    CHECK (interval_seconds >= 5);
