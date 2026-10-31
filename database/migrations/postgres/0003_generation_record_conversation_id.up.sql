-- sdkwork:migration
-- id: 0003_generation_record_conversation_id
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Group generation records into client-scoped multi-turn
--   conversations. The creative composer submits every command of one
--   conversation with the same `conversationId`; without the column the app
--   reload fragmented one conversation into one session per record. The
--   column is nullable: legacy records (and direct API callers that do not
--   group) keep standing alone. Fresh baseline installs get the column via
--   the baseline update.
-- reversible: true
-- rollback: down-migration
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE generation_record
    ADD COLUMN IF NOT EXISTS conversation_id TEXT;

CREATE INDEX IF NOT EXISTS generation_record_tenant_conversation_idx
    ON generation_record (tenant_id, conversation_id, created_at DESC);

COMMIT;
