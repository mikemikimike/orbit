-- Reusable environment mappings shared across worker profiles.
CREATE TABLE IF NOT EXISTS env_mappings (
    id          uuid PRIMARY KEY,
    org_id      uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        text NOT NULL,
    description text NOT NULL DEFAULT '',
    mappings    jsonb NOT NULL DEFAULT '[]'::jsonb,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, name)
);

CREATE TABLE IF NOT EXISTS worker_profile_env_mappings (
    profile_id      uuid NOT NULL REFERENCES worker_profiles(id) ON DELETE CASCADE,
    env_mapping_id  uuid NOT NULL REFERENCES env_mappings(id) ON DELETE CASCADE,
    PRIMARY KEY (profile_id, env_mapping_id)
);

CREATE INDEX IF NOT EXISTS env_mappings_org_idx ON env_mappings (org_id);
CREATE INDEX IF NOT EXISTS worker_profile_env_mappings_profile_idx
    ON worker_profile_env_mappings (profile_id);
