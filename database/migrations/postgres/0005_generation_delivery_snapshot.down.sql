-- sdkwork:migration
-- id: 0005_generation_delivery_snapshot
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Remove the generation delivery policy snapshot table.
-- reversible: true
-- rollback: down-migration
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

DROP TABLE IF EXISTS generation_delivery;

COMMIT;
