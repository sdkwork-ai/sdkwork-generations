-- sdkwork:migration
-- id: 0004_generation_record_owner_list_index
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Index the owner-scoped generation history page. The app list
--   filters by `tenant_id` and `user_id` and orders by `(created_at, id)`
--   descending; without this composite index the predicate falls back to the
--   tenant/conversation index and scans every account's records in the tenant.
-- reversible: true
-- rollback: down-migration
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

CREATE INDEX IF NOT EXISTS generation_record_tenant_user_created_idx
    ON generation_record (tenant_id, user_id, created_at DESC, id DESC);

COMMIT;
