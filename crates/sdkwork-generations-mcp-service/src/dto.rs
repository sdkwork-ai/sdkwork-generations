//! Tool input/output DTOs for the generations MCP service.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Input for `generation.image.create`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateImageInput {
    /// Text prompt describing the image to generate.
    pub prompt: String,
    /// Vendor slug (`openai`, `nano-banana`, `vidu`, `kling`, `volcengine`).
    #[serde(default)]
    pub vendor: Option<String>,
    /// Provider model id or catalog key.
    #[serde(default)]
    pub model: Option<String>,
    /// Requested image size, for example `1024x1024`.
    #[serde(default)]
    pub size: Option<String>,
    /// Requested aspect ratio, for example `1:1`.
    #[serde(default)]
    pub aspect_ratio: Option<String>,
    /// Number of images to generate.
    #[serde(default)]
    pub image_count: Option<i64>,
    /// Requested quality tier when the vendor supports it.
    #[serde(default)]
    pub quality: Option<String>,
    /// Reference image URLs (turns the command into an image edit).
    #[serde(default)]
    pub reference_images: Vec<String>,
}

/// Input for `generation.video.create`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateVideoInput {
    /// Text prompt describing the video to generate.
    pub prompt: String,
    /// Vendor slug (`openai`, `kling`, `vidu`, `volcengine`).
    #[serde(default)]
    pub vendor: Option<String>,
    /// Provider model id or catalog key.
    #[serde(default)]
    pub model: Option<String>,
    /// Requested clip duration in seconds.
    #[serde(default)]
    pub duration_seconds: Option<i64>,
    /// Requested aspect ratio, for example `16:9`.
    #[serde(default)]
    pub aspect_ratio: Option<String>,
    /// Requested resolution, for example `1080p`.
    #[serde(default)]
    pub resolution: Option<String>,
    /// Reference image URLs (first frame for image-to-video).
    #[serde(default)]
    pub reference_images: Vec<String>,
    /// Tail frame image URL for start-end to video vendors.
    #[serde(default)]
    pub last_frame: Option<String>,
}

/// Input for `generation.speech.create`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesizeSpeechInput {
    /// Text to synthesize.
    pub text: String,
    /// Voice identifier when the vendor supports selection.
    #[serde(default)]
    pub voice: Option<String>,
    /// Audio response format, for example `mp3` or `wav`.
    #[serde(default)]
    pub response_format: Option<String>,
    /// Speech speed multiplier.
    #[serde(default)]
    pub speed: Option<f64>,
    /// Provider speech model id.
    #[serde(default)]
    pub model: Option<String>,
}

/// Input for `generation.music.create`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateMusicInput {
    /// Music description prompt (or lyrics when `lyrics` is absent).
    pub prompt: String,
    /// Style tags, for example `pop, upbeat`.
    #[serde(default)]
    pub tags: Option<String>,
    /// Track title.
    #[serde(default)]
    pub title: Option<String>,
    /// Explicit lyrics for lyrics-to-music generation.
    #[serde(default)]
    pub lyrics: Option<String>,
    /// Requested track duration in seconds.
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    /// Negative tags to steer the vendor away from styles.
    #[serde(default)]
    pub negative_tags: Option<String>,
    /// Provider music model id.
    #[serde(default)]
    pub model: Option<String>,
}

/// Input for retrieval tools.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRetrieveInput {
    /// Generation id returned by the create tool.
    pub generation_id: String,
}

/// Typed tool output shared by all generation tools.
///
/// Generation records and results are embedded as JSON values so the MCP
/// schema stays stable while the domain models evolve.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerationsToolOutput {
    /// Persisted generation record (status, ids, provider task id).
    pub generation: serde_json::Value,
    /// Persisted results for the generation.
    pub results: Vec<serde_json::Value>,
    /// Convenience list of media URLs extracted from the results.
    pub media_urls: Vec<String>,
}

/// Error payload returned by failed tool calls.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct GenerationsMcpToolError {
    /// Platform error code.
    pub code: i32,
    /// Human-readable error message.
    pub message: String,
}
