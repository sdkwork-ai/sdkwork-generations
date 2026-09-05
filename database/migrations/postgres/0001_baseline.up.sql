-- sdkwork:migration
-- id: 0001_baseline
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Baseline schema for generations module — creates all 8 core tables
--   (generation_record, generation_record_source_ref, generation_dispatch_job,
--   generation_source_inbox_event, generation_timeline_event, generation_result,
--   generation_record_projection, generation_outbox_event) and their indexes.
--   Statements mirror the consolidated baseline DDL in
--   ddl/baseline/postgres/0001_generations_baseline.sql and are idempotent
--   (IF NOT EXISTS), so replaying them after a baseline install is a no-op.
-- reversible: true
-- rollback: down-migration
-- transactional: true
-- lock: lightweight
-- lock_timeout: 5s
-- statement_timeout: 60s

BEGIN;

CREATE TABLE IF NOT EXISTS generation_record (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    user_id TEXT NOT NULL,
    modality TEXT NOT NULL,
    operation_type TEXT NOT NULL,
    source_provider TEXT NOT NULL,
    source_job_id TEXT,
    idempotency_key TEXT,
    prompt_hash TEXT,
    prompt_preview TEXT,
    input_refs_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    parameter_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL,
    favorite BOOLEAN NOT NULL DEFAULT FALSE,
    result_count INTEGER NOT NULL DEFAULT 0,
    error_code TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMPTZ,
    CHECK (modality IN ('image', 'video', 'music', 'voice', 'audio', 'sfx')),
    CHECK (status IN ('queued', 'running', 'requires_action', 'succeeded', 'failed', 'canceled')),
    CHECK (result_count >= 0)
);

CREATE TABLE IF NOT EXISTS generation_record_source_ref (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    source_provider TEXT NOT NULL,
    source_resource_type TEXT NOT NULL,
    source_resource_id TEXT NOT NULL,
    source_status TEXT,
    source_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS generation_dispatch_job (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    source_provider TEXT NOT NULL,
    operation_type TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    lease_owner TEXT,
    lease_expires_at TIMESTAMPTZ,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_error_code TEXT,
    last_error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (status IN ('pending', 'leased', 'sent', 'retrying', 'succeeded', 'failed', 'canceled')),
    CHECK (attempt_count >= 0),
    CHECK (max_attempts >= 1)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_generation_dispatch_job_idempotency
    ON generation_dispatch_job (tenant_id, source_provider, operation_type, idempotency_key);
CREATE INDEX IF NOT EXISTS idx_generation_dispatch_job_lease
    ON generation_dispatch_job (status, lease_expires_at, priority, created_at);
CREATE INDEX IF NOT EXISTS idx_generation_dispatch_job_retry
    ON generation_dispatch_job (status, next_attempt_at, priority, created_at);

CREATE TABLE IF NOT EXISTS generation_source_inbox_event (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    source_provider TEXT NOT NULL,
    source_event_id TEXT NOT NULL,
    source_job_id TEXT,
    generation_id TEXT,
    event_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'received',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    received_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at TIMESTAMPTZ,
    error_code TEXT,
    error_message TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_generation_source_inbox_provider_event
    ON generation_source_inbox_event (source_provider, source_event_id);
CREATE INDEX IF NOT EXISTS idx_generation_source_inbox_status
    ON generation_source_inbox_event (status, received_at);

CREATE TABLE IF NOT EXISTS generation_timeline_event (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    message TEXT,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_generation_timeline_generation_created
    ON generation_timeline_event (generation_id, created_at, id);

CREATE TABLE IF NOT EXISTS generation_result (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    result_type TEXT NOT NULL,
    ordinal INTEGER NOT NULL DEFAULT 0,
    drive_space_id TEXT,
    drive_node_id TEXT,
    drive_uri TEXT,
    media_resource_id TEXT,
    resource_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    asset_id TEXT,
    preview_text TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (ordinal >= 0)
);

CREATE INDEX IF NOT EXISTS idx_generation_result_generation_ordinal
    ON generation_result (generation_id, ordinal, created_at);
CREATE INDEX IF NOT EXISTS idx_generation_result_asset
    ON generation_result (tenant_id, asset_id)
    WHERE asset_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS generation_record_projection (
    generation_id TEXT PRIMARY KEY REFERENCES generation_record(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    user_id TEXT NOT NULL,
    modality TEXT NOT NULL,
    operation_type TEXT NOT NULL,
    status TEXT NOT NULL,
    favorite BOOLEAN NOT NULL DEFAULT FALSE,
    title TEXT,
    prompt_preview TEXT,
    thumbnail_drive_node_id TEXT,
    latest_result_asset_id TEXT,
    result_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_generation_projection_user_updated
    ON generation_record_projection (tenant_id, user_id, updated_at DESC, generation_id);
CREATE INDEX IF NOT EXISTS idx_generation_projection_user_status_updated
    ON generation_record_projection (tenant_id, user_id, status, updated_at DESC, generation_id);
CREATE INDEX IF NOT EXISTS idx_generation_projection_user_modality_updated
    ON generation_record_projection (tenant_id, user_id, modality, updated_at DESC, generation_id);

CREATE TABLE IF NOT EXISTS generation_outbox_event (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'pending',
    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    published_at TIMESTAMPTZ,
    CHECK (status IN ('pending', 'publishing', 'published', 'failed')),
    CHECK (attempt_count >= 0)
);

CREATE INDEX IF NOT EXISTS idx_generation_outbox_status
    ON generation_outbox_event (status, next_attempt_at, created_at);

COMMIT;
