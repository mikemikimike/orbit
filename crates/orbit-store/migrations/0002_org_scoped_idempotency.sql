-- Idempotency keys are scoped per organization, not globally.
ALTER TABLE executions DROP CONSTRAINT IF EXISTS executions_idempotency_key_key;
CREATE UNIQUE INDEX IF NOT EXISTS executions_org_idempotency_idx
    ON executions (org_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
