-- Scheduling hardening: track consecutive failures and auto-disable.
ALTER TABLE triggers
    ADD COLUMN IF NOT EXISTS consecutive_failures integer NOT NULL DEFAULT 0;
ALTER TABLE triggers
    ADD COLUMN IF NOT EXISTS max_failures integer NOT NULL DEFAULT 5;
