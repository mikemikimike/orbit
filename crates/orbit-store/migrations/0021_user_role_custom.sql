-- Custom roles are stored by name on users.role, so the original enum-shaped check
-- constraint no longer applies. Validation happens in the API (built-in role or an
-- existing custom role for the same organization).
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_role_check;
