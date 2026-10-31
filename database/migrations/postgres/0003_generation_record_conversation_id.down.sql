-- sdkwork:migration
-- id: 0003_generation_record_conversation_id
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Remove the client-scoped conversation grouping column and its
--   tenant/conversation listing index.
-- reversible: true
-- rollback: down-migration
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

DROP INDEX IF EXISTS generation_projection_tenant_conversation_idx;
DROP INDEX IF EXISTS generation_record_tenant_conversation_idx;

ALTER TABLE generation_record_projection
    DROP COLUMN IF EXISTS conversation_id;

ALTER TABLE generation_record
    DROP COLUMN IF EXISTS conversation_id;

COMMIT;
