-- 0.4.x: allow the renamed harness-managed provider kind.
-- The 0.4.0 rename updated existing rows but left the original CHECK constraint in place.
ALTER TABLE providers DROP CONSTRAINT IF EXISTS providers_kind_check;
ALTER TABLE providers
    ADD CONSTRAINT providers_kind_check
    CHECK (kind IN ('openai_compatible', 'harness_managed'));
