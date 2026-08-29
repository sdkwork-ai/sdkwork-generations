//! Domain models for the generations bounded context.
//!
//! All models are aligned with the OpenAPI schema defined in
//! `sdks/sdkwork-generations-app-sdk/openapi/sdkwork-generations-app-api.openapi.json`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Generation modality enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GenerationModality {
    Image,
    Video,
    Music,
    Audio,
    Sfx,
    Voice,
}

impl std::fmt::Display for GenerationModality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image => write!(f, "image"),
            Self::Video => write!(f, "video"),
            Self::Music => write!(f, "music"),
            Self::Audio => write!(f, "audio"),
            Self::Sfx => write!(f, "sfx"),
            Self::Voice => write!(f, "voice"),
        }
    }
}

impl GenerationModality {
    /// Parse a modality from its string representation.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "image" => Some(Self::Image),
            "video" => Some(Self::Video),
            "music" => Some(Self::Music),
            "audio" => Some(Self::Audio),
            "sfx" => Some(Self::Sfx),
            "voice" => Some(Self::Voice),
            _ => None,
        }
    }
}

/// Generation status enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStatus {
    Queued,
    Running,
    RequiresAction,
    Succeeded,
    Failed,
    Canceled,
}

impl std::fmt::Display for GenerationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Running => write!(f, "running"),
            Self::RequiresAction => write!(f, "requires_action"),
            Self::Succeeded => write!(f, "succeeded"),
            Self::Failed => write!(f, "failed"),
            Self::Canceled => write!(f, "canceled"),
        }
    }
}

impl GenerationStatus {
    /// Parse a status from its string representation.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "requires_action" => Some(Self::RequiresAction),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Core records
// ---------------------------------------------------------------------------

/// Core generation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRecord {
    pub id: String,
    #[serde(rename = "tenantId")]
    pub tenant_id: String,
    #[serde(rename = "organizationId")]
    pub organization_id: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: String,
    pub modality: GenerationModality,
    #[serde(rename = "operationType")]
    pub operation_type: String,
    #[serde(rename = "sourceProvider")]
    pub source_provider: Option<String>,
    #[serde(rename = "sourceJobId")]
    pub source_job_id: Option<String>,
    #[serde(rename = "promptPreview")]
    pub prompt_preview: Option<String>,
    pub status: GenerationStatus,
    pub favorite: bool,
    #[serde(rename = "resultCount")]
    pub result_count: i32,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// Generation result record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub id: String,
    #[serde(rename = "generationId")]
    pub generation_id: String,
    #[serde(rename = "resultType")]
    pub result_type: String,
    #[serde(rename = "driveSpaceId")]
    pub drive_space_id: Option<String>,
    #[serde(rename = "driveNodeId")]
    pub drive_node_id: Option<String>,
    #[serde(rename = "driveUri")]
    pub drive_uri: Option<String>,
    #[serde(rename = "resourceSnapshot")]
    pub resource_snapshot: Option<MediaResource>,
    #[serde(rename = "assetId")]
    pub asset_id: Option<String>,
    #[serde(rename = "previewText")]
    pub preview_text: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// Generation timeline event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTimelineEvent {
    pub id: String,
    #[serde(rename = "generationId")]
    pub generation_id: String,
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub message: Option<String>,
    pub payload: Option<serde_json::Value>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// Media resource descriptor attached to a generation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaResource {
    #[serde(rename = "mediaResourceId")]
    pub media_resource_id: Option<String>,
    /// Media kind (for example `image`, `video`, `audio`).
    pub kind: Option<String>,
    /// Provenance of the media (for example `generated`, `external_url`).
    pub source: Option<String>,
    /// Canonical media URL.
    pub url: Option<String>,
    /// Public CDN URL when it differs from [`MediaResource::url`].
    #[serde(rename = "publicUrl")]
    pub public_url: Option<String>,
    /// Provider or drive URI for the media.
    pub uri: Option<String>,
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// Media duration in milliseconds (int64 as decimal string on the wire).
    #[serde(
        rename = "durationMs",
        with = "sdkwork_utils_rust::serde_int64::option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub duration_ms: Option<i64>,
    /// Media size in bytes (int64 as decimal string on the wire).
    #[serde(
        rename = "sizeBytes",
        with = "sdkwork_utils_rust::serde_int64::option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub size_bytes: Option<i64>,
    #[serde(rename = "checksumSha256")]
    pub checksum_sha256: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Request models
// ---------------------------------------------------------------------------

/// Request body for creating a generation command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGenerationCommandRequest {
    #[serde(rename = "tenantId")]
    pub tenant_id: String,
    #[serde(rename = "organizationId")]
    pub organization_id: Option<String>,
    pub prompt: String,
    pub model: Option<String>,
    #[serde(rename = "inputAssetIds")]
    pub input_asset_ids: Option<Vec<String>>,
    pub parameters: Option<serde_json::Value>,
}

/// Request body for generation actions (cancel / retry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationActionRequest {
    pub reason: Option<String>,
}

/// Request body for favoriting a generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteGenerationRequest {
    pub favorite: bool,
}

/// Request body for saving a generation result to assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGenerationResultToAssetsRequest {
    #[serde(rename = "tenantId")]
    pub tenant_id: String,
    #[serde(rename = "collectionId")]
    pub collection_id: Option<String>,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Response models
// ---------------------------------------------------------------------------

/// Response returned when a generation command is accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationCommandResponse {
    pub generation: GenerationRecord,
}

/// Page info for cursor-based pagination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub mode: String,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
}

impl PageInfo {
    /// Create a new cursor page info.
    pub fn cursor(next_cursor: Option<String>, has_more: bool) -> Self {
        Self {
            mode: "cursor".to_string(),
            next_cursor,
            has_more,
        }
    }
}

/// Paginated response of generation records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRecordPage {
    pub items: Vec<GenerationRecord>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// Paginated response of generation results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResultPage {
    pub items: Vec<GenerationResult>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// Paginated response of timeline events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTimelineEventPage {
    pub items: Vec<GenerationTimelineEvent>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

// ---------------------------------------------------------------------------
// List query parameters
// ---------------------------------------------------------------------------

/// Query parameters for listing generation records.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListGenerationsQuery {
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<i32>,
    pub status: Option<String>,
    pub modality: Option<String>,
    #[serde(rename = "operation_type")]
    pub operation_type: Option<String>,
    pub q: Option<String>,
}

/// Query parameters for listing generation results.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListResultsQuery {
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<i32>,
}

/// Query parameters for listing timeline events.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListTimelineQuery {
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<i32>,
}
