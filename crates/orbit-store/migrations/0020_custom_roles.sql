-- Enterprise custom roles: a named permission set per organization. Built-in roles
-- (owner/admin/operator/viewer) remain virtual and are not stored here.
CREATE TABLE IF NOT EXISTS custom_roles (
    id          uuid PRIMARY KEY,
    org_id      uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        text NOT NULL,
    permissions text[] NOT NULL DEFAULT '{}',
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, name)
);

CREATE INDEX IF NOT EXISTS custom_roles_org_idx ON custom_roles (org_id);
