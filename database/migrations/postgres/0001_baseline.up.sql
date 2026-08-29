-- sdkwork:migration
-- id: 0001_baseline
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Baseline schema for generations module — creates all 8 core tables
--   (generation_record, generation_record_source_ref, generation_dispatch_job,
--   generation_source_inbox_event, generation_timeline_event, generation_result,
--   generation_record_projection, generation_outbox_event) and their indexes.
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
    user_id TEXT NOT NULL,
    modality TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    operation_type TEXT NOT NULL,
    prompt TEXT,
    source_refs JSONB DEFAULT '[]'::jsonb,
    result_refs JSONB DEFAULT '[]'::jsonb,
    metadata JSONB DEFAULT '{}'::jsonb,
    config JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS generation_record_source_ref (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    source_provider TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS generation_dispatch_job (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    source_provider TEXT NOT NULL,
    operation_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    attempt_count INTEGER NOT NULL DEFAULT 0,
    lease_owner TEXT,
    lease_expires_at TIMESTAMPTZ,
    next_attempt_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS generation_source_inbox_event (
    id TEXT PRIMARY KEY,
    source_provider TEXT NOT NULL,
    source_event_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS generation_timeline_event (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS generation_result (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    result_type TEXT NOT NULL,
    asset_id TEXT,
    uri TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS generation_record_projection (
    generation_id TEXT PRIMARY KEY REFERENCES generation_record(id) ON DELETE CASCADE,
    latest_status TEXT NOT NULL,
    result_count INTEGER NOT NULL DEFAULT 0,
    last_event_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS generation_outbox_event (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload JSONB DEFAULT '{}'::jsonb,
    published BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at TIMESTAMPTZ
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_generation_record_tenant ON generation_record(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_generation_record_status ON generation_record(status);
CREATE INDEX IF NOT EXISTS idx_generation_record_modality ON generation_record(modality);
CREATE INDEX IF NOT EXISTS idx_generation_dispatch_job_status ON generation_dispatch_job(status, next_attempt_at);
CREATE INDEX IF NOT EXISTS idx_generation_dispatch_job_lease ON generation_dispatch_job(lease_owner, lease_expires_at);
CREATE INDEX IF NOT EXISTS idx_generation_source_inbox_status ON generation_source_inbox_event(status);
CREATE INDEX IF NOT EXISTS idx_generation_timeline_gen ON generation_timeline_event(generation_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_generation_result_gen ON generation_result(generation_id);
CREATE INDEX IF NOT EXISTS idx_generation_outbox_unpub ON generation_outbox_event(published, created_at);

COMMIT;
