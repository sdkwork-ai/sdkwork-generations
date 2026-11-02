-- sdkwork:migration
-- id: 0004_generation_record_owner_list_index
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Remove the owner-scoped generation history index.
-- reversible: true
-- rollback: down-migration
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

DROP INDEX IF EXISTS generation_record_tenant_user_created_idx;

COMMIT;
