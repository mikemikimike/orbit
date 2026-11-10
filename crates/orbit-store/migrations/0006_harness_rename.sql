-- 0.4.0: clean-cut rename from runtime to harness.
ALTER TABLE runtimes RENAME TO harnesses;
ALTER TABLE executions RENAME COLUMN runtime_kind TO harness_kind;
ALTER TABLE agent_instances RENAME COLUMN runtime_id TO harness_id;
ALTER TABLE worker_profiles RENAME COLUMN runtime_kind TO harness_kind;
ALTER TABLE worker_heartbeats RENAME COLUMN runtimes TO harnesses;

-- Agent definitions: spec.runtime -> spec.harness and model management runtime -> harness.
UPDATE agent_versions
SET spec = (spec - 'runtime') || jsonb_build_object('harness', spec->'runtime')
WHERE spec ? 'runtime';

UPDATE agent_versions
SET spec = jsonb_set(spec, '{model,management}', '"harness"')
WHERE spec #>> '{model,management}' = 'runtime';

-- Provider kind rename (harness-managed providers).
UPDATE providers SET kind = 'harness_managed' WHERE kind = 'runtime_managed';

-- API token scopes.
UPDATE api_tokens SET scopes = array_replace(scopes, 'runtimes:read', 'harnesses:read');
