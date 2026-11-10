-- Worker liveness and orphaned-execution recovery.
CREATE TABLE IF NOT EXISTS worker_heartbeats (
    worker_id        text PRIMARY KEY,
    runtimes         text[] NOT NULL DEFAULT '{}',
    hostname         text,
    started_at       timestamptz NOT NULL DEFAULT now(),
    last_heartbeat_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS worker_heartbeats_last_seen_idx
    ON worker_heartbeats (last_heartbeat_at);
