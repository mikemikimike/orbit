CREATE TABLE IF NOT EXISTS skills (
    id          uuid PRIMARY KEY,
    org_id      uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    slug        text NOT NULL,
    name        text NOT NULL,
    description text NOT NULL DEFAULT '',
    content     text NOT NULL,
    files       jsonb NOT NULL DEFAULT '{}'::jsonb,
    version     integer NOT NULL DEFAULT 1,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, slug)
);

CREATE TABLE IF NOT EXISTS worker_profiles (
    id                uuid PRIMARY KEY,
    org_id            uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name              text NOT NULL,
    runtime_kind      text NOT NULL,
    target            text NOT NULL DEFAULT 'docker'
                      CHECK (target IN ('local', 'docker', 'kubernetes')),
    execution_driver  text NOT NULL DEFAULT 'process'
                      CHECK (execution_driver IN ('process', 'docker')),
    image             text NOT NULL DEFAULT '',
    image_tag         text NOT NULL DEFAULT 'latest',
    agent_ids         uuid[] NOT NULL DEFAULT '{}',
    skill_slugs       text[] NOT NULL DEFAULT '{}',
    provider          text,
    model             text,
    auth_secret_name  text,
    env               jsonb NOT NULL DEFAULT '{}'::jsonb,
    resources         jsonb NOT NULL DEFAULT '{}'::jsonb,
    enabled           boolean NOT NULL DEFAULT true,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, name)
);
