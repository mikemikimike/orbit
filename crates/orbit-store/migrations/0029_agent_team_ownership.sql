-- Agent Teams become shareable resources like agents/connections (ADR 0011).
ALTER TABLE agent_teams
    ADD COLUMN IF NOT EXISTS owner_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS owner_team_id uuid REFERENCES access_teams(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS visibility text NOT NULL DEFAULT 'org';

UPDATE agent_teams SET owner_user_id = created_by WHERE owner_user_id IS NULL;

DROP TRIGGER IF EXISTS agent_teams_default_owner_team ON agent_teams;
CREATE TRIGGER agent_teams_default_owner_team
    BEFORE INSERT ON agent_teams
    FOR EACH ROW EXECUTE FUNCTION assign_default_owner_team();
