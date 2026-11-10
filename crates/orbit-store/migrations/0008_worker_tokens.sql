-- 0.5.0: scoped worker tokens replace database/NATS/master-key bootstrap.
CREATE TABLE IF NOT EXISTS worker_tokens (
    id           uuid PRIMARY KEY,
    org_id       uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    profile_id   uuid REFERENCES worker_profiles(id) ON DELETE CASCADE,
    name         text NOT NULL,
    token_prefix text NOT NULL,
    token_hash   text NOT NULL UNIQUE,
    harnesses    text[] NOT NULL DEFAULT '{}',
    expires_at   timestamptz,
    last_used_at timestamptz,
    revoked_at   timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS worker_tokens_org_idx ON worker_tokens (org_id);
CREATE INDEX IF NOT EXISTS worker_tokens_profile_idx ON worker_tokens (profile_id);
