//! Video generation vendor adapters.
//!
//! Supported vendor surfaces: OpenAI video (create/extend), Kling video
//! generation, Vidu text/image/start-end to video, and Volcengine content
//! generation tasks.

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

use crate::gateway::MediaSdkGateway;
use crate::usage::{usage_from_media, usage_from_vidu_creations, MediaUsageKind};
use crate::vendor::{resolve_vendor, GenerationCommandInputs};
use crate::{failed_outcome, pending_outcome, record_outcome, status_from_vendor, succeeded_outcome, task_event};

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
        match selection.vendor.as_str() {
            "openai" => dispatch_openai(self, record, &inputs).await,
            "kling" => dispatch_kling(self, record, &inputs).await,
            "vidu" => dispatch_vidu(self, record, &inputs).await,
            "volcengine" | "jimeng" => dispatch_volcengine(self, record, &inputs).await,
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
            "volcengine" => {
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
