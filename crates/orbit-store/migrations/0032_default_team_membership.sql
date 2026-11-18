-- Every existing organization user joins the Default access team. With team isolation,
-- Default-owned legacy resources remain visible to all members, while non-default teams
-- stay isolated.
INSERT INTO team_members (team_id, user_id, role)
SELECT t.id, u.id, 'member'
FROM access_teams t
JOIN users u ON u.org_id = t.org_id
WHERE t.slug = 'default'
ON CONFLICT (team_id, user_id) DO NOTHING;
