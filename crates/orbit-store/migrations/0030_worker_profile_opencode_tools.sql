-- OpenCode tool permissions controlled per worker profile. Headless runs default to
-- denying built-in Web Search/WebFetch, which otherwise block on interactive input.
ALTER TABLE worker_profiles
    ADD COLUMN IF NOT EXISTS allow_web_search boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS allow_web_fetch boolean NOT NULL DEFAULT false;
