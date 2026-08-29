//! MCP server handler exposing generation tools.

use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        ErrorData, Implementation, ListPromptsResult, ListResourcesResult, ServerCapabilities,
        ServerInfo,
    },
    tool, tool_handler, tool_router, Json, ServerHandler,
};
use serde_json::Value;

use crate::dto::{
    GenerateImageInput, GenerateMusicInput, GenerateVideoInput, GenerationRetrieveInput,
    GenerationsMcpToolError, SynthesizeSpeechInput,
};
use crate::{generation_payload, modality_or_image, tool_error};
use crate::port::GenerationsMcpPort;

/// MCP service exposing generation capabilities to chat agents.
#[derive(Clone)]
pub struct GenerationsMcpService {
    port: Arc<dyn GenerationsMcpPort>,
    tool_router: ToolRouter<Self>,
}

impl GenerationsMcpService {
    /// Build the MCP service over a generations port.
    pub fn new(port: Arc<dyn GenerationsMcpPort>) -> Self {
        Self {
            port,
            tool_router: Self::tool_router(),
        }
    }

    /// List the tools this service exposes.
    pub fn tools(&self) -> Vec<rmcp::model::Tool> {
        self.tool_router.list_all()
    }
}

#[tool_router]
impl GenerationsMcpService {
    #[tool(
        name = "generation.image.create",
        description = "Generate images (text-to-image or image edit when reference images are provided)."
    )]
    async fn image_create(
        &self,
        Parameters(input): Parameters<GenerateImageInput>,
    ) -> Result<Json<crate::dto::GenerationsToolOutput>, Json<GenerationsMcpToolError>> {
        let mut parameters = serde_json::Map::new();
        if let Some(vendor) = input.vendor.as_deref() {
            parameters.insert("vendor".to_string(), Value::String(vendor.to_string()));
        }
        if let Some(aspect_ratio) = input.aspect_ratio.as_deref() {
            parameters.insert(
                "generationConfig".to_string(),
                serde_json::json!({
                    "aspectRatio": aspect_ratio,
                    "imageCount": input.image_count.unwrap_or(1),
                    "quality": input.quality,
                }),
            );
        } else if input.image_count.is_some() || input.quality.is_some() {
            parameters.insert(
                "generationConfig".to_string(),
                serde_json::json!({
                    "imageCount": input.image_count.unwrap_or(1),
                    "quality": input.quality,
                }),
            );
        }
        if input.size.is_some() {
            parameters.insert("size".to_string(), serde_json::json!(input.size));
        }
        if !input.reference_images.is_empty() {
            parameters.insert(
                "referenceImages".to_string(),
                serde_json::json!(input
                    .reference_images
                    .iter()
                    .map(|url| serde_json::json!({ "url": url }))
                    .collect::<Vec<_>>()),
            );
        }
        let operation_type = if input.reference_images.is_empty() {
            "text_to_image"
        } else {
            "image_edit"
        };
        let command = crate::create_command(
            &tenant_id(),
            &input.prompt,
            input.model.as_deref(),
            Value::Object(parameters),
            None,
        );
        let record = self
            .port
            .create_generation(modality_or_image("image"), operation_type, &command)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        let results = self
            .results(&record.id)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        Ok(Json(generation_payload(&record, &results)))
    }

    #[tool(
        name = "generation.image.retrieve",
        description = "Retrieve an image generation (status plus result URLs) by generation id."
    )]
    async fn image_retrieve(
        &self,
        Parameters(input): Parameters<GenerationRetrieveInput>,
    ) -> Result<Json<crate::dto::GenerationsToolOutput>, Json<GenerationsMcpToolError>> {
        self.retrieve(input.generation_id).await
    }

    #[tool(
        name = "generation.video.create",
        description = "Generate videos (text-to-video, image-to-video, or start-end frame interpolation)."
    )]
    async fn video_create(
        &self,
        Parameters(input): Parameters<GenerateVideoInput>,
    ) -> Result<Json<crate::dto::GenerationsToolOutput>, Json<GenerationsMcpToolError>> {
        let mut parameters = serde_json::Map::new();
        if let Some(vendor) = input.vendor.as_deref() {
            parameters.insert("vendor".to_string(), Value::String(vendor.to_string()));
        }
        if input.duration_seconds.is_some()
            || input.aspect_ratio.is_some()
            || input.resolution.is_some()
        {
            parameters.insert(
                "generationConfig".to_string(),
                serde_json::json!({
                    "durationSeconds": input.duration_seconds,
                    "aspectRatio": input.aspect_ratio,
                    "resolution": input.resolution,
                }),
            );
        }
        if !input.reference_images.is_empty() {
            parameters.insert(
                "referenceImages".to_string(),
                serde_json::json!(input
                    .reference_images
                    .iter()
                    .map(|url| serde_json::json!({ "url": url }))
                    .collect::<Vec<_>>()),
            );
        }
        if let Some(last_frame) = input.last_frame.as_deref() {
            parameters.insert("imageTail".to_string(), Value::String(last_frame.to_string()));
        }
        let operation_type = if input.reference_images.is_empty() {
            "text_to_video"
        } else {
            "image_to_video"
        };
        let command = crate::create_command(
            &tenant_id(),
            &input.prompt,
            input.model.as_deref(),
            Value::Object(parameters),
            None,
        );
        let record = self
            .port
            .create_generation(modality_or_image("video"), operation_type, &command)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        let results = self
            .results(&record.id)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        Ok(Json(generation_payload(&record, &results)))
    }

    #[tool(
        name = "generation.video.retrieve",
        description = "Retrieve a video generation (status plus result URLs) by generation id."
    )]
    async fn video_retrieve(
        &self,
        Parameters(input): Parameters<GenerationRetrieveInput>,
    ) -> Result<Json<crate::dto::GenerationsToolOutput>, Json<GenerationsMcpToolError>> {
        self.retrieve(input.generation_id).await
    }

    #[tool(
        name = "generation.speech.create",
        description = "Synthesize speech audio from text (text-to-speech)."
    )]
    async fn speech_create(
        &self,
        Parameters(input): Parameters<SynthesizeSpeechInput>,
    ) -> Result<Json<crate::dto::GenerationsToolOutput>, Json<GenerationsMcpToolError>> {
        let mut parameters = serde_json::Map::new();
        if let Some(voice) = input.voice.as_deref() {
            parameters.insert("voice".to_string(), Value::String(voice.to_string()));
        }
        if let Some(format) = input.response_format.as_deref() {
            parameters.insert(
                "responseFormat".to_string(),
                Value::String(format.to_string()),
            );
        }
        if let Some(speed) = input.speed {
            parameters.insert("speed".to_string(), serde_json::json!(speed));
        }
        let command = crate::create_command(
            &tenant_id(),
            &input.text,
            input.model.as_deref(),
            Value::Object(parameters),
            None,
        );
        let record = self
            .port
            .create_generation(modality_or_image("voice"), "speech", &command)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        let results = self
            .results(&record.id)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        Ok(Json(generation_payload(&record, &results)))
    }

    #[tool(
        name = "generation.music.create",
        description = "Generate music tracks (text-to-music or lyrics-to-music)."
    )]
    async fn music_create(
        &self,
        Parameters(input): Parameters<GenerateMusicInput>,
    ) -> Result<Json<crate::dto::GenerationsToolOutput>, Json<GenerationsMcpToolError>> {
        let mut parameters = serde_json::Map::new();
        if let Some(tags) = input.tags.as_deref() {
            parameters.insert("tags".to_string(), Value::String(tags.to_string()));
        }
        if let Some(title) = input.title.as_deref() {
            parameters.insert("title".to_string(), Value::String(title.to_string()));
        }
        if let Some(lyrics) = input.lyrics.as_deref() {
            parameters.insert("lyrics".to_string(), Value::String(lyrics.to_string()));
        }
        if let Some(duration) = input.duration_seconds {
            parameters.insert(
                "generationConfig".to_string(),
                serde_json::json!({ "durationSeconds": duration }),
            );
        }
        if let Some(negative_tags) = input.negative_tags.as_deref() {
            parameters.insert(
                "negativeTags".to_string(),
                Value::String(negative_tags.to_string()),
            );
        }
        let operation_type = if input.lyrics.is_some() {
            "lyrics_to_music"
        } else {
            "text_to_music"
        };
        let command = crate::create_command(
            &tenant_id(),
            &input.prompt,
            input.model.as_deref(),
            Value::Object(parameters),
            None,
        );
        let record = self
            .port
            .create_generation(modality_or_image("music"), operation_type, &command)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        let results = self
            .results(&record.id)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        Ok(Json(generation_payload(&record, &results)))
    }

    #[tool(
        name = "generation.music.retrieve",
        description = "Retrieve a music generation (status plus track URLs) by generation id."
    )]
    async fn music_retrieve(
        &self,
        Parameters(input): Parameters<GenerationRetrieveInput>,
    ) -> Result<Json<crate::dto::GenerationsToolOutput>, Json<GenerationsMcpToolError>> {
        self.retrieve(input.generation_id).await
    }
}

impl GenerationsMcpService {
    async fn retrieve(
        &self,
        generation_id: String,
    ) -> Result<Json<crate::dto::GenerationsToolOutput>, Json<GenerationsMcpToolError>> {
        let record = self
            .port
            .get_generation(&generation_id)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        let results = self
            .results(&record.id)
            .await
            .map_err(|error| Json(tool_error(&error)))?;
        Ok(Json(generation_payload(&record, &results)))
    }

    async fn results(&self, generation_id: &str) -> Result<Vec<sdkwork_intelligence_generations_service::domain::models::GenerationResult>, sdkwork_intelligence_generations_service::error::GenerationsError> {
        let (items, _) = self
            .port
            .list_results(
                generation_id,
                sdkwork_intelligence_generations_service::ports::ListResultsParams {
                    generation_id: generation_id.to_string(),
                    cursor: None,
                    page_size: Some(20),
                },
            )
            .await?;
        Ok(items)
    }
}

fn tenant_id() -> String {
    std::env::var("GENERATIONS_MCP_TENANT_ID").unwrap_or_else(|_| "0".to_string())
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for GenerationsMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "sdkwork-generations-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Media generation tools: generation.image.create, generation.video.create, \
                 generation.speech.create, generation.music.create. Async vendors return a \
                 generation id; use the matching retrieve tool to poll for finished results.",
            )
    }

    async fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult::default())
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::default())
    }
}
