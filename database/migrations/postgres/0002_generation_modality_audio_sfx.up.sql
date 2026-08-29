-- sdkwork:migration
-- id: 0002_generation_modality_audio_sfx
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Extend the generation_record modality CHECK constraint to accept
--   the audio and sfx modalities that the app-api contract (voice/speech,
--   voice/transcription, voice/translation, sound_effects) already exposes
--   and that the provider adapters now dispatch. Fresh baseline installs
--   get the extended constraint via the baseline update.
-- reversible: true
-- rollback: down-migration
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

ALTER TABLE generation_record
    DROP CONSTRAINT IF EXISTS generation_record_modality_check;
ALTER TABLE generation_record
    ADD CONSTRAINT generation_record_modality_check
    CHECK (modality IN ('image', 'video', 'music', 'voice', 'audio', 'sfx'));

COMMIT;
