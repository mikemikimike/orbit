ALTER TABLE worker_profiles
    ADD COLUMN IF NOT EXISTS native_tool_permissions jsonb NOT NULL DEFAULT '{}'::jsonb;
