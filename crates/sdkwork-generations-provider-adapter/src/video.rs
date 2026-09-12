//! Video generation vendor adapters.
//!
//! Supported vendor surfaces: OpenAI video (create/extend), Kling video
//! generation, Vidu text/image/start-end to video, Volcengine content
//! generation tasks, and Google Veo (`generateVideos` long-running
//! operations).

use std::sync::Arc;

use async_trait::async_trait;
use cloudrouter_open_sdk::models::{
    KlingVideoGenerationRequest, OpenAiVideoCreateRequest, OpenAiVideoExtendRequest,
    ProviderGeneratedMedia, ViduImageToVideoRequest, ViduStartEndToVideoRequest,
    ViduTextToVideoRequest, VolcengineContentGenerationTaskCreateRequest, VolcengineContentPart,
};
use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
use sdkwork_intelligence_generations_service::domain::models::{
    CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationResult,
    GenerationStatus, MediaResource,
};
use sdkwork_intelligence_generations_service::error::GenerationsError;
use sdkwork_intelligence_generations_service::ports::{
    GenerationDispatchOutcome, GenerationProvider, GenerationUsage,
};

use crate::gateway::{GeminiVideoGenerationRequest, GeminiVideoInstance, GeminiVideoParameters, MediaSdkGateway};
use crate::usage::{usage_from_media, usage_from_vidu_creations, MediaUsageKind};
use crate::vendor::{resolve_vendor, GenerationCommandInputs};
use crate::{failed_outcome, pending_outcome, record_outcome, status_from_vendor, succeeded_outcome, task_event, with_resolved_vendor};

/// Video generation provider dispatching through the media gateway.
pub struct VideoGenerationProviderAdapter {
    gateway: Arc<dyn MediaSdkGateway>,
    default_vendor: String,
}

impl VideoGenerationProviderAdapter {
    /// Create a video provider bound to a media gateway.
    pub fn new(gateway: Arc<dyn MediaSdkGateway>, default_vendor: impl Into<String>) -> Self {
        Self {
            gateway,
            default_vendor: default_vendor.into(),
        }
    }
}

#[async_trait]
impl GenerationProvider for VideoGenerationProviderAdapter {
    fn modality(&self) -> GenerationModality {
        GenerationModality::Video
    }

    fn operation_types(&self) -> Vec<&str> {
        vec!["text_to_video", "image_to_video", "video_extend"]
    }

    fn vendor(&self) -> &str {
        &self.default_vendor
    }

    async fn dispatch(
        &self,
        record: &GenerationRecord,
        command: &CreateGenerationCommandRequest,
        _context: &GenerationsRequestContext,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let selection = resolve_vendor(command, &self.default_vendor);
        let inputs = GenerationCommandInputs::from_command(command);
        // The refresh path routes polling by record.source_provider; persist
        // the resolved vendor so the same surface that dispatched the task
        // also polls it, even when it differs from the adapter default.
        let record = with_resolved_vendor(record, &selection.vendor);
        match selection.vendor.as_str() {
            "openai" => dispatch_openai(self, &record, &inputs).await,
            "nano-banana" | "veo" => dispatch_gemini(self, &record, &inputs).await,
            "kling" => dispatch_kling(self, &record, &inputs).await,
            "vidu" => dispatch_vidu(self, &record, &inputs).await,
            "volcengine" | "jimeng" => dispatch_volcengine(self, &record, &inputs).await,
            other => Err(GenerationsError::Provider(format!(
                "video vendor {other:?} is not supported by the generations provider adapter"
            ))),
        }
    }

    async fn retrieve(
        &self,
        record: &GenerationRecord,
        _context: &GenerationsRequestContext,
    ) -> Result<Option<GenerationDispatchOutcome>, GenerationsError> {
        let Some(task_id) = record.source_job_id.as_deref().filter(|v| !v.trim().is_empty()) else {
            return Ok(None);
        };
        let inputs = GenerationCommandInputs::default();
        match record.source_provider.as_deref().unwrap_or_default() {
            "openai" => {
                let video = self
                    .gateway
                    .openai_retrieve_video(task_id)
                    .await
                    .map_err(|error| GenerationsError::Provider(error.to_string()))?;
                Ok(Some(outcome_from_openai_video(record, &video)))
            }
            "nano-banana" | "veo" | "google" | "gemini" => {
                let operation = self
                    .gateway
                    .gemini_retrieve_video_operation(task_id)
                    .await
                    .map_err(|error| GenerationsError::Provider(error.to_string()))?;
                Ok(Some(outcome_from_gemini_operation(record, &operation, &inputs)))
            }
            "kling" => {
                let task = self
                    .gateway
                    .kling_retrieve_video_generation(task_id)
                    .await
                    .map_err(|error| GenerationsError::Provider(error.to_string()))?;
                Ok(Some(outcome_from_media_task(
                    record,
                    "kling",
                    task.task_id.as_deref().or(task.id.as_deref()),
                    task.status.as_deref().or(task.state.as_deref()),
                    task.videos.clone().unwrap_or_default(),
                    task.error.as_ref().map(crate::task_error_message),
                )))
            }
            "vidu" => {
                let task = self
                    .gateway
                    .vidu_retrieve_video_creations(task_id)
                    .await
                    .map_err(|error| GenerationsError::Provider(error.to_string()))?;
                let results = vidu_results(record, &task);
                let usage = usage_from_vidu_creations("vidu", &inputs, &task);
                Ok(Some(match status_from_vendor(task.state.as_deref()) {
                    GenerationStatus::Succeeded => {
                        succeeded_outcome(record, results, Some(usage), Vec::new())
                    }
                    GenerationStatus::Failed => {
                        failed_outcome(record, "vidu video task failed")
                    }
                    _ => pending_outcome(record, task_id, Vec::new()),
                }))
            }
            "volcengine" | "jimeng" => {
                let task = self
                    .gateway
                    .volcengine_retrieve_video_task(task_id)
                    .await
                    .map_err(|error| GenerationsError::Provider(error.to_string()))?;
                let media = task
                    .result
                    .as_ref()
                    .and_then(|result| result.videos.clone())
                    .or_else(|| task.videos.clone())
                    .unwrap_or_default();
                Ok(Some(outcome_from_media_task(
                    record,
                    "volcengine",
                    task.task_id.as_deref().or(task.id.as_deref()),
                    task.status.as_deref().or(task.state.as_deref()),
                    media,
                    task.error.as_ref().map(crate::task_error_message),
                )))
            }
            _ => Ok(None),
        }
    }
}

async fn dispatch_openai(
    adapter: &VideoGenerationProviderAdapter,
    record: &GenerationRecord,
    inputs: &GenerationCommandInputs,
) -> Result<GenerationDispatchOutcome, GenerationsError> {
    if record.operation_type == "video_extend" {
        let request = OpenAiVideoExtendRequest {
            image: inputs.first_reference_image(),
            metadata: None,
            model: (!inputs.model.is_empty()).then(|| inputs.model.clone()),
            prompt: Some(inputs.prompt.clone()).filter(|value| !value.is_empty()),
            seconds: inputs.duration_seconds.map(|value| value as i64),
            size: inputs.size.clone(),
            video: inputs.first_reference_image(),
        };
        let video = adapter
            .gateway
            .openai_create_video_extension(&request)
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        return Ok(outcome_from_openai_video(record, &video));
    }
    if record.operation_type == "image_to_video" && inputs.first_reference_image().is_none() {
        return Err(GenerationsError::InvalidInput(
            "image_to_video requires a reference image".to_string(),
        ));
    }
    let request = OpenAiVideoCreateRequest {
        image: inputs.first_reference_image(),
        metadata: None,
        model: model_or_default(inputs, "sora-2"),
        prompt: inputs.prompt.clone(),
        seconds: inputs.duration_seconds.map(|value| value as i64),
        size: inputs.size.clone(),
        video: None,
    };
    let video = adapter
        .gateway
        .openai_create_video(&request)
        .await
        .map_err(|error| GenerationsError::Provider(error.to_string()))?;
    Ok(outcome_from_openai_video(record, &video))
}

async fn dispatch_gemini(
    adapter: &VideoGenerationProviderAdapter,
    record: &GenerationRecord,
    inputs: &GenerationCommandInputs,
) -> Result<GenerationDispatchOutcome, GenerationsError> {
    if record.operation_type == "video_extend" {
        return Err(GenerationsError::InvalidInput(
            "veo does not support video_extend".to_string(),
        ));
    }
    if record.operation_type == "image_to_video" {
        return Err(GenerationsError::InvalidInput(
            "veo image_to_video requires base64-encoded image input, which generation commands do not carry; use text_to_video".to_string(),
        ));
    }
    let model = model_or_default(inputs, "veo-3.0-generate-001");
    let parameters = GeminiVideoParameters {
        aspect_ratio: inputs.aspect_ratio.clone(),
        duration_seconds: inputs.duration_seconds.map(|value| value as i64),
        person_generation: None,
    };
    let request = GeminiVideoGenerationRequest {
        instances: vec![GeminiVideoInstance {
            prompt: inputs.prompt.clone(),
            image: None,
        }],
        parameters: Some(parameters),
    };
    let operation = adapter
        .gateway
        .gemini_create_video_generation(&model, &request)
        .await
        .map_err(|error| GenerationsError::Provider(error.to_string()))?;
    Ok(outcome_from_gemini_operation(record, &operation, inputs))
}

/// Builds the dispatch outcome for a Veo long-running operation envelope:
/// finished with media becomes a succeeded outcome, a finished error becomes
/// a failure, and anything else stays pending on the operation name.
fn outcome_from_gemini_operation(
    record: &GenerationRecord,
    operation: &crate::gateway::GeminiVideoOperation,
    inputs: &GenerationCommandInputs,
) -> GenerationDispatchOutcome {
    if operation.done {
        if let Some(error) = operation.error.as_ref() {
            return failed_outcome(
                record,
                &error
                    .message
                    .clone()
                    .unwrap_or_else(|| "veo video operation failed".to_string()),
            );
        }
        let uris = operation.video_uris();
        if uris.is_empty() {
            return failed_outcome(
                record,
                &operation.filtered_reason()
                    .unwrap_or_else(|| "veo video operation finished without media".to_string()),
            );
        }
        let results = uris
            .iter()
            .map(|uri| video_result(record, uri))
            .collect();
        let mut usage = GenerationUsage::new("google");
        usage.model = record
            .source_job_id
            .as_deref()
            .and_then(extract_model_from_operation_name);
        usage.video_seconds = inputs.duration_seconds.unwrap_or(0.0);
        succeeded_outcome(record, results, Some(usage), Vec::new())
    } else {
        let Some(operation_name) = operation
            .operation_name()
            .map(str::to_owned)
            .or_else(|| record.source_job_id.clone())
        else {
            return failed_outcome(record, "veo operation response is missing name");
        };
        pending_outcome(record, &operation_name, vec![task_event(record, &operation_name)])
    }
}

/// Extracts the model id from a Veo operation resource name
/// (`models/{model}/operations/{operation}`).
fn extract_model_from_operation_name(operation_name: &str) -> Option<String> {
    let trimmed = operation_name.trim().trim_start_matches('/');
    let model = trimmed
        .strip_prefix("models/")?
        .split('/')
        .next()?
        .trim();
    (!model.is_empty()).then(|| model.to_owned())
}

async fn dispatch_kling(
    adapter: &VideoGenerationProviderAdapter,
    record: &GenerationRecord,
    inputs: &GenerationCommandInputs,
) -> Result<GenerationDispatchOutcome, GenerationsError> {
    let request = KlingVideoGenerationRequest {
        aspect_ratio: inputs.aspect_ratio.clone(),
        callback_url: None,
        cfg_scale: inputs.cfg_scale,
        duration: inputs.duration_seconds.map(|value| value as i64),
        image: inputs.first_reference_image(),
        image_tail: inputs.reference_image_tail.clone(),
        mode: inputs.mode.clone(),
        model: (!inputs.model.is_empty()).then(|| inputs.model.clone()),
        negative_prompt: inputs.negative_prompt.clone(),
        prompt: inputs.prompt.clone(),
    };
    let task = adapter
        .gateway
        .kling_create_video_generation(&request)
        .await
        .map_err(|error| GenerationsError::Provider(error.to_string()))?;
    finish_media_task_dispatch(
        record,
        "kling",
        inputs,
        task.task_id.as_deref().or(task.id.as_deref()),
        task.status.as_deref().or(task.state.as_deref()),
        task.videos.clone().unwrap_or_default(),
        task.error.as_ref().map(crate::task_error_message),
    )
}

async fn dispatch_vidu(
    adapter: &VideoGenerationProviderAdapter,
    record: &GenerationRecord,
    inputs: &GenerationCommandInputs,
) -> Result<GenerationDispatchOutcome, GenerationsError> {
    let model = model_or_default(inputs, "viduq1");
    let task = match record.operation_type.as_str() {
        "text_to_video" => {
            let request = ViduTextToVideoRequest {
                aspect_ratio: inputs.aspect_ratio.clone(),
                callback_url: None,
                duration: inputs.duration_seconds.map(|value| value as i64),
                model: model.clone(),
                movement_amplitude: None,
                payload: None,
                prompt: inputs.prompt.clone(),
                resolution: inputs.resolution.clone(),
                seed: inputs.seed,
            };
            adapter
                .gateway
                .vidu_create_text_to_video(&request)
                .await
                .map_err(|error| GenerationsError::Provider(error.to_string()))?
        }
        "image_to_video" => {
            if inputs.reference_images.is_empty() {
                return Err(GenerationsError::InvalidInput(
                    "vidu image_to_video requires at least one reference image".to_string(),
                ));
            }
            let request = ViduImageToVideoRequest {
                aspect_ratio: inputs.aspect_ratio.clone(),
                callback_url: None,
                duration: inputs.duration_seconds.map(|value| value as i64),
                images: inputs.reference_images.clone(),
                model: model.clone(),
                movement_amplitude: None,
                payload: None,
                prompt: Some(inputs.prompt.clone()).filter(|value| !value.is_empty()),
                resolution: inputs.resolution.clone(),
                seed: inputs.seed,
            };
            adapter
                .gateway
                .vidu_create_image_to_video(&request)
                .await
                .map_err(|error| GenerationsError::Provider(error.to_string()))?
        }
        other => {
            if other != "video_extend" {
                return Err(GenerationsError::InvalidInput(format!(
                    "vidu does not support operation {other:?}"
                )));
            }
            if inputs.reference_images.len() < 2 {
                return Err(GenerationsError::InvalidInput(
                    "vidu start-end video requires first and last frame references".to_string(),
                ));
            }
            let mut images = inputs.reference_images.clone();
            let image_tail = images.pop();
            let request = ViduStartEndToVideoRequest {
                aspect_ratio: inputs.aspect_ratio.clone(),
                callback_url: None,
                duration: inputs.duration_seconds.map(|value| value as i64),
                images,
                model: model.clone(),
                movement_amplitude: None,
                payload: None,
                prompt: Some(inputs.prompt.clone()).filter(|value| !value.is_empty()),
                resolution: inputs.resolution.clone(),
                seed: inputs.seed,
            };
            let _ = image_tail;
            adapter
                .gateway
                .vidu_create_start_end_to_video(&request)
                .await
                .map_err(|error| GenerationsError::Provider(error.to_string()))?
        }
    };
    let Some(task_id) = task.task_id.as_deref().filter(|value| !value.trim().is_empty()) else {
        return Err(GenerationsError::Provider(
            "vidu task response is missing task_id".to_string(),
        ));
    };
    Ok(pending_outcome(record, task_id, vec![task_event(record, task_id)]))
}

async fn dispatch_volcengine(
    adapter: &VideoGenerationProviderAdapter,
    record: &GenerationRecord,
    inputs: &GenerationCommandInputs,
) -> Result<GenerationDispatchOutcome, GenerationsError> {
    let mut content = vec![VolcengineContentPart {
        file_id: None,
        image_url: None,
        text: Some(inputs.prompt.clone()),
        video_url: None,
        r#type: "text".to_string(),
    }];
    if let Some(reference) = inputs.first_reference_image() {
        content.insert(
            0,
            VolcengineContentPart {
                file_id: None,
                image_url: Some(reference),
                text: None,
                video_url: None,
                r#type: "image_url".to_string(),
            },
        );
    }
    let request = VolcengineContentGenerationTaskCreateRequest {
        callback_url: None,
        content,
        metadata: None,
        model: model_or_default(inputs, "doubao-seedance-1-0-lite-t2v-250428"),
    };
    let response = adapter
        .gateway
        .volcengine_create_video_task(&request)
        .await
        .map_err(|error| GenerationsError::Provider(error.to_string()))?;
    let Some(task_id) = response
        .task_id
        .as_deref()
        .or(response.id.as_deref())
        .filter(|value| !value.trim().is_empty())
    else {
        return Err(GenerationsError::Provider(
            "volcengine task response is missing task_id".to_string(),
        ));
    };
    Ok(pending_outcome(record, task_id, vec![task_event(record, task_id)]))
}

fn outcome_from_openai_video(
    record: &GenerationRecord,
    video: &cloudrouter_open_sdk::models::OpenAiVideo,
) -> GenerationDispatchOutcome {
    let status = status_from_openai_video_status(&video.status);
    match status {
        GenerationStatus::Succeeded => {
            let url = video
                .content_url
                .clone()
                .or_else(|| video.url.clone())
                .unwrap_or_default();
            let results = if url.is_empty() {
                Vec::new()
            } else {
                vec![video_result(record, &url)]
            };
            let mut usage = GenerationUsage::new("openai");
            usage.model = video.model.clone();
            usage.video_seconds = video.seconds.unwrap_or_default() as f64;
            succeeded_outcome(record, results, Some(usage), Vec::new())
        }
        GenerationStatus::Failed => failed_outcome(record, "openai video generation failed"),
        _ => {
            let mut running = record.clone();
            running.status = status;
            running.source_job_id = Some(video.id.clone());
            record_outcome(running)
        }
    }
}

fn outcome_from_media_task(
    record: &GenerationRecord,
    vendor: &str,
    task_id: Option<&str>,
    status: Option<&str>,
    media: Vec<ProviderGeneratedMedia>,
    error: Option<String>,
) -> GenerationDispatchOutcome {
    match status_from_vendor(status) {
        GenerationStatus::Succeeded => {
            let results = media
                .iter()
                .filter_map(|entry| entry.url.clone().or_else(|| entry.uri.clone()))
                .map(|url| video_result(record, &url))
                .collect();
            let usage = usage_from_media(vendor, &GenerationCommandInputs::default(), &media, MediaUsageKind::Video);
            succeeded_outcome(record, results, Some(usage), Vec::new())
        }
        GenerationStatus::Failed => {
            failed_outcome(record, &error.unwrap_or_else(|| format!("{vendor} video task failed")))
        }
        _ => pending_outcome(
            record,
            task_id.unwrap_or_default(),
            Vec::new(),
        ),
    }
}

fn finish_media_task_dispatch(
    record: &GenerationRecord,
    vendor: &str,
    inputs: &GenerationCommandInputs,
    task_id: Option<&str>,
    status: Option<&str>,
    media: Vec<ProviderGeneratedMedia>,
    error: Option<String>,
) -> Result<GenerationDispatchOutcome, GenerationsError> {
    let Some(task_id) = task_id.filter(|value| !value.trim().is_empty()) else {
        return Err(GenerationsError::Provider(
            "vendor task response is missing task_id".to_string(),
        ));
    };
    match status_from_vendor(status) {
        GenerationStatus::Succeeded => {
            let results = media
                .iter()
                .filter_map(|entry| entry.url.clone().or_else(|| entry.uri.clone()))
                .map(|url| video_result(record, &url))
                .collect();
            let usage = usage_from_media(vendor, inputs, &media, MediaUsageKind::Video);
            Ok(succeeded_outcome(record, results, Some(usage), Vec::new()))
        }
        GenerationStatus::Failed => Err(GenerationsError::Provider(error.unwrap_or_else(
            || format!("vendor task {task_id} failed"),
        ))),
        _ => Ok(pending_outcome(record, task_id, vec![task_event(record, task_id)])),
    }
}

fn vidu_results(
    record: &GenerationRecord,
    task: &cloudrouter_open_sdk::models::ViduTaskCreationsResponse,
) -> Vec<GenerationResult> {
    task.creations
        .iter()
        .flatten()
        .filter_map(|creation| {
            creation
                .video_url
                .clone()
                .or_else(|| creation.url.clone())
        })
        .map(|url| video_result(record, &url))
        .collect()
}

fn status_from_openai_video_status(status: &str) -> GenerationStatus {
    match status.trim().to_ascii_lowercase().as_str() {
        "completed" | "succeeded" => GenerationStatus::Succeeded,
        "failed" => GenerationStatus::Failed,
        "queued" | "pending" => GenerationStatus::Queued,
        _ => GenerationStatus::Running,
    }
}

fn video_result(record: &GenerationRecord, url: &str) -> GenerationResult {
    GenerationResult {
        id: format!("{}:video-{}", record.id, uuid::Uuid::new_v4()),
        generation_id: record.id.clone(),
        result_type: "video".to_string(),
        drive_space_id: None,
        drive_node_id: None,
        drive_uri: None,
        resource_snapshot: Some(MediaResource {
            media_resource_id: None,
            kind: Some("video".to_string()),
            source: Some("generated".to_string()),
            url: Some(url.to_string()),
            public_url: Some(url.to_string()),
            uri: Some(url.to_string()),
            media_type: Some("video".to_string()),
            content_type: Some("video/mp4".to_string()),
            width: None,
            height: None,
            duration_ms: None,
            size_bytes: None,
            checksum_sha256: None,
            metadata: None,
        }),
        asset_id: None,
        preview_text: record.prompt_preview.clone(),
        created_at: crate::now_iso(),
    }
}

fn model_or_default(inputs: &GenerationCommandInputs, default: &str) -> String {
    if inputs.model.trim().is_empty() {
        default.to_string()
    } else {
        inputs.model.clone()
    }
}

#[cfg(test)]
mod video_lifecycle_tests {
    use std::sync::Arc;

    use cloudrouter_open_sdk::models::{
        KlingVideoGenerationTask, OpenAiVideo, ProviderGeneratedMedia,
    };
    use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
    use sdkwork_intelligence_generations_service::domain::models::{
        CreateGenerationCommandRequest, GenerationModality, GenerationStatus,
    };
    use sdkwork_intelligence_generations_service::ports::GenerationProvider;

    use crate::gateway::{
        test_support::ScriptedGateway, GeminiVideoAsset, GeminiVideoGenerateResponse,
        GeminiVideoOperation, GeminiVideoOperationResponse, GeminiVideoSample,
    };
    use crate::video::VideoGenerationProviderAdapter;

    const OPERATION_NAME: &str = "models/veo-3.0-generate-001/operations/op-1";

    fn sample_record(operation_type: &str) -> sdkwork_intelligence_generations_service::domain::models::GenerationRecord {
        sdkwork_intelligence_generations_service::domain::models::GenerationRecord {
            id: "gen-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            user_id: "user-1".to_string(),
            modality: GenerationModality::Video,
            operation_type: operation_type.to_string(),
            source_provider: Some("openai".to_string()),
            source_job_id: None,
            prompt_preview: Some("gen".to_string()),
            status: GenerationStatus::Queued,
            favorite: false,
            result_count: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn command(model: &str) -> CreateGenerationCommandRequest {
        CreateGenerationCommandRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "a drone shot over rice terraces".to_string(),
            model: Some(model.to_string()),
            input_asset_ids: None,
            parameters: None,
        }
    }

    fn context() -> GenerationsRequestContext {
        GenerationsRequestContext::from_parts(
            "tenant-1".to_string(),
            "user-1".to_string(),
            "trace-1".to_string(),
        )
    }

    fn pending_operation() -> GeminiVideoOperation {
        GeminiVideoOperation {
            name: Some(OPERATION_NAME.to_string()),
            done: false,
            response: None,
            error: None,
        }
    }

    fn finished_operation() -> GeminiVideoOperation {
        GeminiVideoOperation {
            name: Some(OPERATION_NAME.to_string()),
            done: true,
            response: Some(GeminiVideoOperationResponse {
                generate_video_response: Some(GeminiVideoGenerateResponse {
                    generated_samples: Some(vec![GeminiVideoSample {
                        video: Some(GeminiVideoAsset {
                            uri: Some("https://cdn.example/veo.mp4".to_string()),
                            url: None,
                        }),
                        gcs_uri: None,
                        uri: None,
                        url: None,
                    }]),
                    videos: None,
                    rai_media_filtered_reasons: None,
                }),
            }),
            error: None,
        }
    }

    #[tokio::test]
    async fn veo_text_to_video_dispatch_pends_on_operation_name_and_persists_vendor() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.gemini_video_operation.lock().unwrap() = Some(pending_operation());
        let provider = VideoGenerationProviderAdapter::new(gateway.clone(), "openai");

        let outcome = provider
            .dispatch(
                &sample_record("text_to_video"),
                &command("google/veo-3.0-generate-001"),
                &context(),
            )
            .await
            .expect("veo dispatch succeeds");

        let (model, request) = gateway
            .last_gemini_video_request
            .lock()
            .unwrap()
            .clone()
            .expect("gemini request captured");
        assert_eq!(model, "veo-3.0-generate-001");
        assert_eq!(request.instances.len(), 1);
        assert_eq!(request.instances[0].prompt, "a drone shot over rice terraces");

        assert_eq!(outcome.record.status, GenerationStatus::Running);
        assert_eq!(
            outcome.record.source_job_id.as_deref(),
            Some(OPERATION_NAME),
            "the full operation name is the polling identifier"
        );
        assert_eq!(
            outcome.record.source_provider.as_deref(),
            Some("nano-banana"),
            "the resolved vendor replaces the adapter default for refresh routing"
        );
    }

    #[tokio::test]
    async fn veo_retrieve_finished_operation_collects_video_urls() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.gemini_video_operation.lock().unwrap() = Some(finished_operation());
        let provider = VideoGenerationProviderAdapter::new(gateway.clone(), "openai");

        let mut record = sample_record("text_to_video");
        record.source_provider = Some("nano-banana".to_string());
        record.source_job_id = Some(OPERATION_NAME.to_string());
        let outcome = provider
            .retrieve(&record, &context())
            .await
            .expect("veo retrieve succeeds")
            .expect("veo retrieve produces an outcome");

        assert_eq!(
            gateway
                .last_gemini_video_operation_name
                .lock()
                .unwrap()
                .as_deref(),
            Some(OPERATION_NAME),
            "the stored operation name is polled verbatim"
        );
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.results.len(), 1);
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.url.as_deref(), Some("https://cdn.example/veo.mp4"));
    }

    #[tokio::test]
    async fn veo_image_to_video_is_rejected_with_guidance() {
        let provider = VideoGenerationProviderAdapter::new(Arc::new(ScriptedGateway::default()), "openai");
        let mut command = command("google/veo-3.0-generate-001");
        command.parameters = Some(serde_json::json!({
            "referenceImages": [{ "url": "https://cdn.example/first-frame.png" }]
        }));
        let error = provider
            .dispatch(&sample_record("image_to_video"), &command, &context())
            .await
            .expect_err("veo image_to_video must fail closed");
        assert!(error.to_string().contains("base64"), "{error}");
    }

    #[tokio::test]
    async fn veo_safety_filtered_operation_fails_with_reason() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.gemini_video_operation.lock().unwrap() = Some(GeminiVideoOperation {
            name: Some(OPERATION_NAME.to_string()),
            done: true,
            response: Some(GeminiVideoOperationResponse {
                generate_video_response: Some(GeminiVideoGenerateResponse {
                    generated_samples: None,
                    videos: None,
                    rai_media_filtered_reasons: Some(vec!["PROHIBITED_CONTENT".to_string()]),
                }),
            }),
            error: None,
        });
        let provider = VideoGenerationProviderAdapter::new(gateway.clone(), "openai");

        let mut record = sample_record("text_to_video");
        record.source_provider = Some("nano-banana".to_string());
        record.source_job_id = Some(OPERATION_NAME.to_string());
        let outcome = provider
            .retrieve(&record, &context())
            .await
            .expect("veo retrieve succeeds")
            .expect("veo retrieve produces an outcome");
        assert_eq!(outcome.record.status, GenerationStatus::Failed);
    }

    #[tokio::test]
    async fn kling_video_task_lifecycle_pends_then_collects_media() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.kling_video_create_task.lock().unwrap() = Some(KlingVideoGenerationTask {
            task_id: Some("kling-task-7".to_string()),
            state: Some("submitted".to_string()),
            ..Default::default()
        });
        let provider = VideoGenerationProviderAdapter::new(gateway.clone(), "openai");

        let mut kling_command = command("kling/kling-v2-master");
        kling_command.parameters = Some(serde_json::json!({
            "generationConfig": { "durationSeconds": 8 }
        }));
        let created = provider
            .dispatch(
                &sample_record("text_to_video"),
                &kling_command,
                &context(),
            )
            .await
            .expect("kling dispatch pends");
        assert_eq!(created.record.status, GenerationStatus::Running);
        assert_eq!(created.record.source_job_id.as_deref(), Some("kling-task-7"));
        assert_eq!(
            created.record.source_provider.as_deref(),
            Some("kling"),
            "the resolved vendor replaces the adapter default for refresh routing"
        );
        let request = gateway
            .last_kling_video_create_request
            .lock()
            .unwrap()
            .clone()
            .expect("kling request captured");
        assert_eq!(request.prompt, "a drone shot over rice terraces");
        assert_eq!(request.duration, Some(8));

        *gateway.kling_video_retrieve_task.lock().unwrap() = Some(KlingVideoGenerationTask {
            task_id: Some("kling-task-7".to_string()),
            state: Some("succeed".to_string()),
            videos: Some(vec![ProviderGeneratedMedia {
                url: Some("https://cdn.example/kling-final.mp4".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        });
        let outcome = provider
            .retrieve(&created.record, &context())
            .await
            .expect("kling retrieve succeeds")
            .expect("kling retrieve produces an outcome");
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.results.len(), 1);
        assert_eq!(
            outcome.results[0].resource_snapshot.as_ref().unwrap().url.as_deref(),
            Some("https://cdn.example/kling-final.mp4")
        );
    }

    #[tokio::test]
    async fn openai_video_task_lifecycle_pends_then_collects_media() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.openai_video_create.lock().unwrap() = Some(OpenAiVideo {
            id: "video-42".to_string(),
            status: "queued".to_string(),
            model: Some("sora-2".to_string()),
            seconds: Some(8),
            content_url: None,
            url: None,
            ..Default::default()
        });
        let provider = VideoGenerationProviderAdapter::new(gateway.clone(), "openai");

        let created = provider
            .dispatch(
                &sample_record("text_to_video"),
                &command("openai/sora-2"),
                &context(),
            )
            .await
            .expect("openai video dispatch pends");
        assert_eq!(
            created.record.status,
            GenerationStatus::Queued,
            "the vendor 'queued' status maps onto the generation status verbatim"
        );
        assert_eq!(created.record.source_job_id.as_deref(), Some("video-42"));

        *gateway.openai_video_retrieve.lock().unwrap() = Some(OpenAiVideo {
            id: "video-42".to_string(),
            status: "completed".to_string(),
            model: Some("sora-2".to_string()),
            seconds: Some(8),
            content_url: Some("https://cdn.example/sora-final.mp4".to_string()),
            url: None,
            ..Default::default()
        });
        let outcome = provider
            .retrieve(&created.record, &context())
            .await
            .expect("openai video retrieve succeeds")
            .expect("openai video retrieve produces an outcome");
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.results.len(), 1);
        assert_eq!(
            outcome.results[0].resource_snapshot.as_ref().unwrap().url.as_deref(),
            Some("https://cdn.example/sora-final.mp4")
        );
    }
}
