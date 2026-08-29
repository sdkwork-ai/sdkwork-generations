//! Provider port interfaces for the generations domain.
//!
//! Providers encapsulate the external AI service integrations (image, video,
//! music, sfx, voice). Concrete implementations are injected at the assembly
//! layer; the service crate only defines the trait contracts.

use async_trait::async_trait;

use crate::domain::models::{
    CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationResult,
    GenerationTimelineEvent,
};
use crate::error::GenerationsError;

/// Business-level request context passed to providers.
pub use crate::context::GenerationsRequestContext;

/// Metering facts extracted from a provider dispatch or retrieval.
///
/// The quantities mirror the cloudrouter billing meters for media
/// generations: image results, video seconds, audio seconds, and LLM tokens.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GenerationUsage {
    /// Vendor slug that produced the usage (for example `openai`, `kling`).
    pub vendor: String,
    /// Provider model identifier used for the generation.
    pub model: Option<String>,
    /// Number of generated images.
    pub image_count: i64,
    /// Generated video seconds.
    pub video_seconds: f64,
    /// Generated audio seconds (speech, music, sound effects).
    pub audio_seconds: f64,
    /// Prompt tokens reported by the vendor, when available.
    pub input_tokens: i64,
    /// Completion tokens reported by the vendor, when available.
    pub output_tokens: i64,
    /// Raw vendor usage payload for downstream reconciliation.
    pub raw: Option<serde_json::Value>,
}

impl GenerationUsage {
    /// Create an empty usage fact for the given vendor.
    pub fn new(vendor: &str) -> Self {
        Self {
            vendor: vendor.to_string(),
            ..Self::default()
        }
    }

    /// Whether any metered quantity is present.
    pub fn is_empty(&self) -> bool {
        self.image_count == 0
            && self.video_seconds == 0.0
            && self.audio_seconds == 0.0
            && self.input_tokens == 0
            && self.output_tokens == 0
    }
}

/// Billing fact emitted after a dispatch or retrieval persists usage.
#[derive(Debug, Clone)]
pub struct GenerationUsageFact {
    pub generation_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub modality: String,
    pub operation_type: String,
    pub source: GenerationUsageSource,
    pub usage: GenerationUsage,
}

/// Where a usage fact originated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationUsageSource {
    /// Usage captured when the generation command was dispatched.
    Dispatch,
    /// Usage captured when an async provider task was retrieved.
    Retrieve,
}

impl GenerationUsageSource {
    /// Stable wire name used in timeline payloads and outbox events.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dispatch => "dispatch",
            Self::Retrieve => "retrieve",
        }
    }
}

/// Provider dispatch outcome: the updated record plus everything the service
/// must persist (results, timeline events, metering usage).
#[derive(Debug, Clone)]
pub struct GenerationDispatchOutcome {
    /// Provider-assigned record state (status, source job id, result count).
    pub record: GenerationRecord,
    /// Normalized results to persist for the generation.
    pub results: Vec<GenerationResult>,
    /// Timeline events describing the dispatch progress.
    pub timeline_events: Vec<GenerationTimelineEvent>,
    /// Metering facts captured during the dispatch, when available.
    pub usage: Option<GenerationUsage>,
}

impl GenerationDispatchOutcome {
    /// Build an outcome that only carries the updated record.
    pub fn from_record(record: GenerationRecord) -> Self {
        Self {
            record,
            results: Vec::new(),
            timeline_events: Vec::new(),
            usage: None,
        }
    }
}

/// Provider capable of dispatching generation commands to an external AI service.
///
/// Each provider handles one or more modalities and operation types. The
/// service layer routes incoming commands to the appropriate provider based on
/// modality and operation type matching.
#[async_trait]
pub trait GenerationProvider: Send + Sync {
    /// Returns the modality this provider handles.
    fn modality(&self) -> GenerationModality;

    /// Returns the operation types this provider supports.
    fn operation_types(&self) -> Vec<&str>;

    /// Stable vendor slug used as `sourceProvider` on records and in usage.
    fn vendor(&self) -> &str;

    /// Dispatch a generation command to the external service.
    ///
    /// `record` is the persisted generation record (status `queued`). The
    /// outcome carries the provider-assigned fields (sourceJobId, status),
    /// normalized results, timeline events, and usage facts.
    async fn dispatch(
        &self,
        record: &GenerationRecord,
        command: &CreateGenerationCommandRequest,
        context: &GenerationsRequestContext,
    ) -> Result<GenerationDispatchOutcome, GenerationsError>;

    /// Retrieve the latest provider state for an async generation task.
    ///
    /// Returns `Ok(None)` when the provider has nothing to refresh (sync
    /// vendors or terminal records). The service calls this on read paths for
    /// non-terminal records before returning results to the caller.
    async fn retrieve(
        &self,
        record: &GenerationRecord,
        context: &GenerationsRequestContext,
    ) -> Result<Option<GenerationDispatchOutcome>, GenerationsError> {
        let _ = (record, context);
        Ok(None)
    }
}

/// Port for recording generation usage for billing.
///
/// Implementations persist the metering facts (timeline event, outbox event,
/// or a direct billing system call). Failures must not fail the generation.
#[async_trait]
pub trait GenerationUsagePort: Send + Sync {
    /// Record a metering fact for billing.
    async fn record_usage(&self, fact: &GenerationUsageFact) -> Result<(), GenerationsError>;
}

/// Port for saving generation results to the asset catalog.
///
/// Concrete implementations integrate with the sdkwork-assets system to persist
/// generated media as managed assets.
#[async_trait]
pub trait AssetPort: Send + Sync {
    /// Save a generation result to the asset catalog.
    ///
    /// Returns the updated `GenerationResult` with the `assetId` field populated
    /// on success.
    async fn save_generation_result(
        &self,
        generation_id: &str,
        result_id: &str,
        request: &crate::domain::models::SaveGenerationResultToAssetsRequest,
        context: &GenerationsRequestContext,
    ) -> Result<crate::domain::models::GenerationResult, GenerationsError>;
}
