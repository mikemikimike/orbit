-- New shareable resources inherit the organization's Default access team unless a caller
-- explicitly assigns an owner team. Keeps behavior org-wide during the migration.
CREATE OR REPLACE FUNCTION assign_default_owner_team() RETURNS trigger AS $$
BEGIN
    IF NEW.owner_team_id IS NULL THEN
        SELECT id INTO NEW.owner_team_id
        FROM access_teams
        WHERE org_id = NEW.org_id AND slug = 'default';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO $$
DECLARE
    table_name text;
BEGIN
    FOREACH table_name IN ARRAY ARRAY[
        'agents', 'skills', 'connections', 'providers',
        'worker_profiles', 'triggers', 'tool_policies'
    ] LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', table_name || '_default_owner_team', table_name);
        EXECUTE format(
            'CREATE TRIGGER %I BEFORE INSERT ON %I FOR EACH ROW EXECUTE FUNCTION assign_default_owner_team()',
            table_name || '_default_owner_team',
            table_name
        );
    END LOOP;
END $$;
