-- Access teams and resource ownership/visibility (ADR 0011). Behavior stays org-wide
-- until enforcement lands; the default team and owner columns are backfilled.
CREATE TABLE IF NOT EXISTS access_teams (
    id          uuid PRIMARY KEY,
    org_id      uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        text NOT NULL,
    slug        text NOT NULL,
    created_by  uuid REFERENCES users(id) ON DELETE SET NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, slug)
);

CREATE TABLE IF NOT EXISTS team_members (
    team_id    uuid NOT NULL REFERENCES access_teams(id) ON DELETE CASCADE,
    user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role       text NOT NULL DEFAULT 'member' CHECK (role IN ('lead', 'member', 'viewer')),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, user_id)
);

CREATE INDEX IF NOT EXISTS access_teams_org_idx ON access_teams (org_id);
CREATE INDEX IF NOT EXISTS team_members_user_idx ON team_members (user_id);

-- One Default team per existing organization.
INSERT INTO access_teams (id, org_id, name, slug)
SELECT gen_random_uuid(), o.id, 'Default', 'default'
FROM organizations o
WHERE NOT EXISTS (
    SELECT 1 FROM access_teams t WHERE t.org_id = o.id AND t.slug = 'default'
);

-- Existing org owners/admins lead the Default team.
INSERT INTO team_members (team_id, user_id, role)
SELECT t.id, u.id, 'lead'
FROM access_teams t
JOIN users u ON u.org_id = t.org_id
WHERE t.slug = 'default' AND u.role IN ('owner', 'admin')
ON CONFLICT DO NOTHING;

DO $$
DECLARE
    table_name text;
BEGIN
    FOREACH table_name IN ARRAY ARRAY[
        'agents', 'skills', 'connections', 'providers',
        'worker_profiles', 'triggers', 'tool_policies'
    ] LOOP
        EXECUTE format(
            'ALTER TABLE %I ADD COLUMN IF NOT EXISTS owner_user_id uuid REFERENCES users(id) ON DELETE SET NULL',
            table_name
        );
        EXECUTE format(
            'ALTER TABLE %I ADD COLUMN IF NOT EXISTS owner_team_id uuid REFERENCES access_teams(id) ON DELETE SET NULL',
            table_name
        );
        EXECUTE format(
            'ALTER TABLE %I ADD COLUMN IF NOT EXISTS visibility text NOT NULL DEFAULT ''org'' CHECK (visibility IN (''private'', ''team'', ''org''))',
            table_name
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I (org_id, visibility, owner_team_id)',
            table_name || '_visibility_idx',
            table_name
        );
        EXECUTE format(
            'UPDATE %I r SET owner_team_id = t.id FROM access_teams t WHERE t.org_id = r.org_id AND t.slug = ''default'' AND r.owner_team_id IS NULL',
            table_name
        );
    END LOOP;
END $$;

UPDATE agents a SET owner_user_id = a.created_by WHERE a.owner_user_id IS NULL;
UPDATE connections c SET owner_user_id = c.created_by WHERE c.owner_user_id IS NULL;
