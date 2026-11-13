-- Worker profile env var mappings from ORBIT connections/secrets.
ALTER TABLE worker_profiles
    ADD COLUMN IF NOT EXISTS env_mappings jsonb NOT NULL DEFAULT '[]'::jsonb;
