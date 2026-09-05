-- sdkwork:migration
-- id: 0001_baseline
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Rollback baseline schema — drops all 8 core tables and their indexes.
-- reversible: true
-- transactional: true
-- lock: lightweight
-- lock_timeout: 5s
-- statement_timeout: 60s

BEGIN;

DROP INDEX IF EXISTS idx_generation_outbox_status;
DROP INDEX IF EXISTS idx_generation_projection_user_modality_updated;
DROP INDEX IF EXISTS idx_generation_projection_user_status_updated;
DROP INDEX IF EXISTS idx_generation_projection_user_updated;
DROP INDEX IF EXISTS idx_generation_result_asset;
DROP INDEX IF EXISTS idx_generation_result_generation_ordinal;
DROP INDEX IF EXISTS idx_generation_timeline_generation_created;
DROP INDEX IF EXISTS idx_generation_source_inbox_status;
DROP INDEX IF EXISTS ux_generation_source_inbox_provider_event;
DROP INDEX IF EXISTS idx_generation_dispatch_job_retry;
DROP INDEX IF EXISTS idx_generation_dispatch_job_lease;
DROP INDEX IF EXISTS ux_generation_dispatch_job_idempotency;

DROP TABLE IF EXISTS generation_outbox_event;
DROP TABLE IF EXISTS generation_record_projection;
DROP TABLE IF EXISTS generation_result;
DROP TABLE IF EXISTS generation_timeline_event;
DROP TABLE IF EXISTS generation_source_inbox_event;
DROP TABLE IF EXISTS generation_dispatch_job;
DROP TABLE IF EXISTS generation_record_source_ref;
DROP TABLE IF EXISTS generation_record;

COMMIT;
