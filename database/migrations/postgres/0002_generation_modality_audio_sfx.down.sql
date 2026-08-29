-- sdkwork:migration
-- id: 0002_generation_modality_audio_sfx
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Restore the original four-modality CHECK constraint.
-- reversible: true
-- rollback: down-migration
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

DELETE FROM generation_record WHERE modality IN ('audio', 'sfx');

ALTER TABLE generation_record
    DROP CONSTRAINT IF EXISTS generation_record_modality_check;
ALTER TABLE generation_record
    ADD CONSTRAINT generation_record_modality_check
    CHECK (modality IN ('image', 'video', 'music', 'voice'));

COMMIT;
