ALTER TABLE worker_profiles
    ADD COLUMN IF NOT EXISTS allow_glab boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS allow_gh boolean NOT NULL DEFAULT false;
