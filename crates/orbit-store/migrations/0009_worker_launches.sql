-- 0.5.1: launcher-managed worker instances.
CREATE TABLE IF NOT EXISTS worker_launches (
    id              uuid PRIMARY KEY,
    org_id          uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    profile_id      uuid NOT NULL REFERENCES worker_profiles(id) ON DELETE CASCADE,
    name            text NOT NULL,
    target          text NOT NULL,
    status          text NOT NULL DEFAULT 'pending',
    runtime_id      text,
    image           text NOT NULL,
    worker_id       text,
    worker_token_id uuid REFERENCES worker_tokens(id) ON DELETE SET NULL,
    error           text,
    created_by      uuid REFERENCES users(id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    started_at      timestamptz,
    stopped_at      timestamptz
);

CREATE INDEX IF NOT EXISTS worker_launches_org_idx
    ON worker_launches (org_id, created_at DESC);
