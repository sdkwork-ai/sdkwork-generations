//! Storage-center generation asset delivery policy snapshot.
//!
//! The storage center (Cloud Router) owns whether a finished generation is
//! copied into SDKWork Drive and what the vendor asset lifetime is. That policy
//! is resolved **once** while the creating caller's verified dual tokens exist
//! (the app-api resolve endpoint) and then persisted with the generation, so
//! background dispatch/completion applies the snapshot instead of re-resolving
//! with a deployment identity (which would be a cross-tenant read).
//!
//! The default is deliberately cost-saving: `remote_only` keeps the vendor
//! reference only; nothing is copied into Drive unless a policy opts in.

/// Copy the generated media into SDKWork Drive.
pub const DELIVERY_MODE_TO_DRIVE: &str = "to_drive";
/// Keep the vendor reference only (default; cost saving).
pub const DELIVERY_MODE_REMOTE_ONLY: &str = "remote_only";

/// A download deadline defaults to this percentage of the vendor asset TTL.
pub const DEFAULT_DOWNLOAD_DEADLINE_TTL_PERCENT: i32 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationDeliveryMode {
    RemoteOnly,
    ToDrive,
}

impl GenerationDeliveryMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            DELIVERY_MODE_TO_DRIVE => Some(Self::ToDrive),
            DELIVERY_MODE_REMOTE_ONLY => Some(Self::RemoteOnly),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RemoteOnly => DELIVERY_MODE_REMOTE_ONLY,
            Self::ToDrive => DELIVERY_MODE_TO_DRIVE,
        }
    }

    pub fn copies_to_drive(self) -> bool {
        matches!(self, Self::ToDrive)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationDeliveryState {
    Pending,
    Skipped,
    Importing,
    Imported,
    Failed,
    Expired,
}

impl GenerationDeliveryState {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pending" => Some(Self::Pending),
            "skipped" => Some(Self::Skipped),
            "importing" => Some(Self::Importing),
            "imported" => Some(Self::Imported),
            "failed" => Some(Self::Failed),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Skipped => "skipped",
            Self::Importing => "importing",
            Self::Imported => "imported",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }

    /// Terminal states never transition again.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Skipped | Self::Imported | Self::Failed | Self::Expired
        )
    }
}

/// The delivery decision and targets captured for one generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationDeliverySnapshot {
    pub generation_id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub user_id: String,
    pub media_kind: String,
    pub vendor_code: Option<String>,
    pub delivery_mode: GenerationDeliveryMode,
    pub drive_space_id: Option<String>,
    pub drive_folder_node_id: Option<String>,
    /// Policy override for the download deadline, in seconds.
    pub download_deadline_seconds: Option<i32>,
    /// Vendor asset lifetime the storage center reported, in seconds.
    pub vendor_asset_ttl_seconds: Option<i32>,
    /// Scope row the decision came from (`tenant` / `organization` / `user`).
    pub source_scope_type: Option<String>,
    pub source_scope_id: Option<String>,
}

impl GenerationDeliverySnapshot {
    /// The cost-saving default used when no policy is available (no storage
    /// center configured, or a pure-background flow without a caller scope).
    pub fn remote_only_default(
        generation_id: impl Into<String>,
        tenant_id: impl Into<String>,
        organization_id: impl Into<String>,
        user_id: impl Into<String>,
        media_kind: impl Into<String>,
        vendor_code: Option<String>,
    ) -> Self {
        Self {
            generation_id: generation_id.into(),
            tenant_id: tenant_id.into(),
            organization_id: organization_id.into(),
            user_id: user_id.into(),
            media_kind: media_kind.into(),
            vendor_code,
            delivery_mode: GenerationDeliveryMode::RemoteOnly,
            drive_space_id: None,
            drive_folder_node_id: None,
            download_deadline_seconds: None,
            vendor_asset_ttl_seconds: None,
            source_scope_type: None,
            source_scope_id: None,
        }
    }

    /// Deadline the dispatch must finish a Drive download by, in seconds.
    ///
    /// A policy override is clamped to the vendor TTL; without one the default
    /// is 80% of the TTL, so a download always completes before the vendor
    /// asset disappears. No TTL and no override means no deadline.
    pub fn effective_download_deadline_seconds(&self) -> Option<i32> {
        let ttl = self.vendor_asset_ttl_seconds.filter(|seconds| *seconds > 0);
        let policy = self
            .download_deadline_seconds
            .filter(|seconds| *seconds > 0);
        match (policy, ttl) {
            (Some(policy), Some(ttl)) => Some(policy.min(ttl)),
            (Some(policy), None) => Some(policy),
            (None, Some(ttl)) => {
                Some((ttl.saturating_mul(DEFAULT_DOWNLOAD_DEADLINE_TTL_PERCENT) / 100).max(1))
            }
            (None, None) => None,
        }
    }

    /// Absolute epoch-millis expiry of the vendor asset, when a TTL is known.
    pub fn assets_expires_at_epoch_ms(&self, finished_at_epoch_ms: i64) -> Option<i64> {
        self.vendor_asset_ttl_seconds
            .filter(|seconds| *seconds > 0)
            .map(|seconds| finished_at_epoch_ms.saturating_add(i64::from(seconds) * 1_000))
    }

    /// Whether a `remote_only` snapshot is only a retained vendor reference
    /// (i.e. no Drive copy will ever be produced for it).
    pub fn keeps_remote_reference_only(&self) -> bool {
        !self.delivery_mode.copies_to_drive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> GenerationDeliverySnapshot {
        GenerationDeliverySnapshot::remote_only_default(
            "gen-1",
            "100001",
            "7",
            "42",
            "video",
            Some("vidu".to_owned()),
        )
    }

    #[test]
    fn the_default_is_remote_only_and_keeps_the_vendor_reference() {
        let snapshot = snapshot();
        assert_eq!(snapshot.delivery_mode, GenerationDeliveryMode::RemoteOnly);
        assert_eq!(
            snapshot.delivery_mode,
            GenerationDeliveryMode::parse("REMOTE_ONLY").expect("parse")
        );
        assert!(!snapshot.delivery_mode.copies_to_drive());
        assert!(snapshot.keeps_remote_reference_only());
        assert_eq!(snapshot.vendor_code.as_deref(), Some("vidu"));
        assert_eq!(snapshot.effective_download_deadline_seconds(), None);
        assert_eq!(snapshot.assets_expires_at_epoch_ms(1_000), None);
    }

    #[test]
    fn a_policy_deadline_is_clamped_to_the_vendor_ttl() {
        let mut snapshot = snapshot();
        snapshot.vendor_asset_ttl_seconds = Some(3_600);
        assert_eq!(snapshot.effective_download_deadline_seconds(), Some(2_880));
        assert_eq!(snapshot.assets_expires_at_epoch_ms(1_000), Some(3_601_000));

        snapshot.download_deadline_seconds = Some(10_000);
        assert_eq!(
            snapshot.effective_download_deadline_seconds(),
            Some(3_600),
            "the deadline never exceeds the vendor asset lifetime"
        );
        snapshot.download_deadline_seconds = Some(120);
        assert_eq!(snapshot.effective_download_deadline_seconds(), Some(120));
    }

    #[test]
    fn to_drive_is_an_explicit_opt_in() {
        let mut snapshot = snapshot();
        snapshot.delivery_mode = GenerationDeliveryMode::ToDrive;
        assert!(snapshot.delivery_mode.copies_to_drive());
        assert!(!snapshot.keeps_remote_reference_only());
        assert_eq!(snapshot.delivery_mode.as_str(), DELIVERY_MODE_TO_DRIVE);
    }

    #[test]
    fn delivery_states_round_trip_and_expose_terminality() {
        for state in [
            GenerationDeliveryState::Pending,
            GenerationDeliveryState::Skipped,
            GenerationDeliveryState::Importing,
            GenerationDeliveryState::Imported,
            GenerationDeliveryState::Failed,
            GenerationDeliveryState::Expired,
        ] {
            assert_eq!(GenerationDeliveryState::parse(state.as_str()), Some(state));
        }
        assert!(!GenerationDeliveryState::Pending.is_terminal());
        assert!(!GenerationDeliveryState::Importing.is_terminal());
        assert!(GenerationDeliveryState::Imported.is_terminal());
        assert!(GenerationDeliveryState::Expired.is_terminal());
        assert_eq!(GenerationDeliveryState::parse("unknown"), None);
    }
}
