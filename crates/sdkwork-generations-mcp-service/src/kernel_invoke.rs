//! Tool invocation routing for the kernel MCP provider.

use std::sync::{Arc, OnceLock};

use sdkwork_intelligence_generations_service::domain::models::{
    CreateGenerationCommandRequest, GenerationModality,
};

use crate::dto::{
    GenerateImageInput, GenerateMusicInput, GenerateVideoInput, GenerationRetrieveInput,
    SynthesizeSpeechInput,
};
use crate::port::GenerationsMcpPort;

/// Capability document served as the MCP capability resource.
pub const CAPABILITY_DOCUMENT: &str = "\
sdkwork-generations MCP capabilities

Image generation: openai (gpt-image-2 generations/edits), nano-banana (gemini), \
vidu (reference-to-image), kling (image generation), volcengine (doubao seedream).
Video generation: openai (sora create/extend), kling, vidu (text/image/start-end), \
volcengine (seedance).
Speech synthesis: openai (gpt-4o-mini-tts), transcription and translation via whisper.
Music generation: suno (text-to-music and lyrics-to-music).

Vendor dispatch happens through the cloudrouter media gateway (account-pool routed).
Async vendors expose generation.retrieve tools; poll with the returned generation id.
";

/// Invoke a generations tool by its unnamespaced tool name.
pub fn invoke(
    port: &Arc<dyn GenerationsMcpPort>,
    tool_name: &str,
    arguments_json: &str,
) -> Result<String, String> {
    match tool_name {
        "image.create" => create_image(port, arguments_json),
        "image.retrieve" => retrieve(port, arguments_json),
        "video.create" => create_video(port, arguments_json),
        "video.retrieve" => retrieve(port, arguments_json),
        "speech.create" => create_speech(port, arguments_json),
        "music.create" => create_music(port, arguments_json),
        "music.retrieve" => retrieve(port, arguments_json),
        other => Err(format!("generations tool {other:?} is not implemented")),
    }
}

fn create_image(
    port: &Arc<dyn GenerationsMcpPort>,
    arguments_json: &str,
) -> Result<String, String> {
    let input: GenerateImageInput = parse_arguments(arguments_json)?;
    let mut parameters = serde_json::Map::new();
    if let Some(vendor) = input.vendor.as_deref() {
        parameters.insert("vendor".to_string(), serde_json::json!(vendor));
    }
    if input.aspect_ratio.is_some() || input.image_count.is_some() || input.quality.is_some() {
        parameters.insert(
            "generationConfig".to_string(),
            serde_json::json!({
                "aspectRatio": input.aspect_ratio,
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
        serde_json::Value::Object(parameters),
        None,
    );
    run_create(port, GenerationModality::Image, operation_type, &command)
}

fn create_video(
    port: &Arc<dyn GenerationsMcpPort>,
    arguments_json: &str,
) -> Result<String, String> {
    let input: GenerateVideoInput = parse_arguments(arguments_json)?;
    let mut parameters = serde_json::Map::new();
    if let Some(vendor) = input.vendor.as_deref() {
        parameters.insert("vendor".to_string(), serde_json::json!(vendor));
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
        parameters.insert("imageTail".to_string(), serde_json::json!(last_frame));
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
        serde_json::Value::Object(parameters),
        None,
    );
    run_create(port, GenerationModality::Video, operation_type, &command)
}

fn create_speech(
    port: &Arc<dyn GenerationsMcpPort>,
    arguments_json: &str,
) -> Result<String, String> {
    let input: SynthesizeSpeechInput = parse_arguments(arguments_json)?;
    let mut parameters = serde_json::Map::new();
    if let Some(voice) = input.voice.as_deref() {
        parameters.insert("voice".to_string(), serde_json::json!(voice));
    }
    if let Some(format) = input.response_format.as_deref() {
        parameters.insert("responseFormat".to_string(), serde_json::json!(format));
    }
    if let Some(speed) = input.speed {
        parameters.insert("speed".to_string(), serde_json::json!(speed));
    }
    let command = crate::create_command(
        &tenant_id(),
        &input.text,
        input.model.as_deref(),
        serde_json::Value::Object(parameters),
        None,
    );
    run_create(port, GenerationModality::Voice, "speech", &command)
}

fn create_music(
    port: &Arc<dyn GenerationsMcpPort>,
    arguments_json: &str,
) -> Result<String, String> {
    let input: GenerateMusicInput = parse_arguments(arguments_json)?;
    let mut parameters = serde_json::Map::new();
    if let Some(tags) = input.tags.as_deref() {
        parameters.insert("tags".to_string(), serde_json::json!(tags));
    }
    if let Some(title) = input.title.as_deref() {
        parameters.insert("title".to_string(), serde_json::json!(title));
    }
    if let Some(lyrics) = input.lyrics.as_deref() {
        parameters.insert("lyrics".to_string(), serde_json::json!(lyrics));
    }
    if let Some(duration) = input.duration_seconds {
        parameters.insert(
            "generationConfig".to_string(),
            serde_json::json!({ "durationSeconds": duration }),
        );
    }
    if let Some(negative_tags) = input.negative_tags.as_deref() {
        parameters.insert("negativeTags".to_string(), serde_json::json!(negative_tags));
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
        serde_json::Value::Object(parameters),
        None,
    );
    run_create(port, GenerationModality::Music, operation_type, &command)
}

fn retrieve(port: &Arc<dyn GenerationsMcpPort>, arguments_json: &str) -> Result<String, String> {
    let input: GenerationRetrieveInput = parse_arguments(arguments_json)?;
    let generation_id = input.generation_id;
    blocking_runtime().block_on(async move {
        let port = Arc::clone(port);
        let record = port
            .get_generation(&generation_id)
            .await
            .map_err(|error| error.to_string())?;
        let (results, _) = port
            .list_results(
                &generation_id,
                sdkwork_intelligence_generations_service::ports::ListResultsParams {
                    generation_id: generation_id.clone(),
                    cursor: None,
                    page_size: Some(20),
                },
            )
            .await
            .map_err(|error| error.to_string())?;
        Ok(crate::generation_payload(&record, &results))
    })
    .and_then(|payload| serde_json::to_string(&payload).map_err(|error| error.to_string()))
}

fn run_create(
    port: &Arc<dyn GenerationsMcpPort>,
    modality: GenerationModality,
    operation_type: &str,
    command: &CreateGenerationCommandRequest,
) -> Result<String, String> {
    let port = Arc::clone(port);
    let command = command.clone();
    let operation_type = operation_type.to_string();
    blocking_runtime().block_on(async move {
        let record = port
            .create_generation(modality, &operation_type, &command)
            .await
            .map_err(|error| error.to_string())?;
        let (results, _) = port
            .list_results(
                &record.id,
                sdkwork_intelligence_generations_service::ports::ListResultsParams {
                    generation_id: record.id.clone(),
                    cursor: None,
                    page_size: Some(20),
                },
            )
            .await
            .map_err(|error| error.to_string())?;
        Ok(crate::generation_payload(&record, &results))
    })
    .and_then(|payload| serde_json::to_string(&payload).map_err(|error| error.to_string()))
}

fn parse_arguments<T>(arguments_json: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    if arguments_json.trim().is_empty() {
        return Err("tool arguments are required".to_string());
    }
    serde_json::from_str(arguments_json).map_err(|error| format!("invalid tool arguments: {error}"))
}

fn tenant_id() -> String {
    std::env::var("GENERATIONS_MCP_TENANT_ID").unwrap_or_else(|_| "0".to_string())
}

/// Dedicated multi-thread runtime for blocking kernel tool invocation.
fn blocking_runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .expect("generations mcp kernel tokio runtime")
    })
}
