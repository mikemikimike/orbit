-- Platform-wide settings (key/value JSON). Used for OIDC configuration managed
-- from the UI instead of environment variables.
CREATE TABLE IF NOT EXISTS platform_settings (
    key         text PRIMARY KEY,
    value       jsonb NOT NULL,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    updated_by  uuid REFERENCES users(id) ON DELETE SET NULL
);

INSERT INTO platform_settings (key, value)
VALUES ('oidc', '{}'::jsonb)
ON CONFLICT (key) DO NOTHING;
