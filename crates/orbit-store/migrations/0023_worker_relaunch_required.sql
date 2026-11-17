-- A worker profile changed in a way that requires restarting the live worker. Set when
-- the profile is saved without applying the change; cleared on the next launch.
ALTER TABLE worker_launches
    ADD COLUMN IF NOT EXISTS relaunch_required boolean NOT NULL DEFAULT false;
