-- sdkwork:migration
-- id: 0005_generation_delivery_snapshot
-- engine: postgres
-- module: sdkwork-generations
-- purpose: Persist the storage-center delivery policy resolved for a generation.
--
--   The storage center owns whether generated media is copied into SDKWork Drive
--   (`storage_generation_delivery_policy`, vendor TTLs in
--   `storage_vendor_asset_ttl`). The policy is resolved once while the creating
--   user's verified dual tokens exist (app-api resolve endpoint) and stored here:
--   background dispatch/completion then applies the snapshot instead of
--   re-resolving with a deployment identity, which would be a cross-tenant read.
--
--   When no row exists for a generation, the caller applies the cost-saving
--   default: `remote_only` with the vendor reference retained until expiry.
-- reversible: true
-- rollback: down-migration
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

CREATE TABLE IF NOT EXISTS generation_delivery (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generation_record(id) ON DELETE CASCADE,
    tenant_id TEXT NOT NULL,
    organization_id TEXT NOT NULL DEFAULT '0',
    user_id TEXT NOT NULL,
    media_kind TEXT NOT NULL,
    vendor_code TEXT,
    delivery_mode TEXT NOT NULL DEFAULT 'remote_only',
    drive_space_id TEXT,
    drive_folder_node_id TEXT,
    download_deadline_seconds INTEGER,
    vendor_asset_ttl_seconds INTEGER,
    assets_expires_at TIMESTAMPTZ,
    remote_url TEXT,
    delivery_state TEXT NOT NULL DEFAULT 'pending',
    source_scope_type TEXT,
    source_scope_id TEXT,
    policy_resolved_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (delivery_mode IN ('remote_only', 'to_drive')),
    CHECK (delivery_state IN ('pending', 'skipped', 'importing', 'imported', 'failed', 'expired')),
    CHECK (media_kind IN ('all', 'image', 'video', 'audio')),
    CHECK (download_deadline_seconds IS NULL OR download_deadline_seconds > 0),
    CHECK (vendor_asset_ttl_seconds IS NULL OR vendor_asset_ttl_seconds > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_generation_delivery_generation
    ON generation_delivery (generation_id);
CREATE INDEX IF NOT EXISTS idx_generation_delivery_state_expiry
    ON generation_delivery (delivery_state, assets_expires_at);
CREATE INDEX IF NOT EXISTS idx_generation_delivery_tenant_state
    ON generation_delivery (tenant_id, delivery_state);

COMMIT;
