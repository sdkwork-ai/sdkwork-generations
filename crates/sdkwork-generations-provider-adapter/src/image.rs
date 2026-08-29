//! Image generation vendor adapters.
//!
//! Supported vendor surfaces: OpenAI-compatible image generations/edits
//! (including `gpt-image-2`), Google nano-banana task API, Vidu reference to
//! image, Kling image generation, and Volcengine Ark image generation.

use std::sync::Arc;

use async_trait::async_trait;
use cloudrouter_open_sdk::models::{
    NanoBananaImageGenerationRequest, OpenAiImageEditRequest, OpenAiImageGenerationRequest,
    OpenAiImageReferenceInputList, ProviderGeneratedMedia,
    ViduReferenceToImageRequest,
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

use crate::gateway::{MediaSdkGateway, VolcengineImageGenerationRequest};
use crate::{
    failed_outcome, pending_outcome, record_outcome, status_from_vendor, succeeded_outcome,
    task_event,
};
use crate::usage::{usage_from_media, usage_from_openai_image_list, MediaUsageKind};
use crate::vendor::{resolve_vendor, GenerationCommandInputs, VendorSelection};

/// Image generation provider dispatching through the media gateway.
pub struct ImageGenerationProviderAdapter {
    gateway: Arc<dyn MediaSdkGateway>,
    default_vendor: String,
}

impl ImageGenerationProviderAdapter {
    /// Create an image provider bound to a media gateway.
    pub fn new(gateway: Arc<dyn MediaSdkGateway>, default_vendor: impl Into<String>) -> Self {
        Self {
            gateway,
            default_vendor: default_vendor.into(),
        }
    }
}

#[async_trait]
impl GenerationProvider for ImageGenerationProviderAdapter {
    fn modality(&self) -> GenerationModality {
        GenerationModality::Image
    }

    fn operation_types(&self) -> Vec<&str> {
        vec!["text_to_image", "image_edit"]
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
            "openai" | "midjourney" => {
                self::surfaces::dispatch_openai(self, record, &selection, &inputs).await
            }
            "nano-banana" => self::surfaces::dispatch_nano_banana(self, record, &selection, &inputs).await,
            "vidu" => self::surfaces::dispatch_vidu(self, record, &selection, &inputs).await,
            "kling" => self::surfaces::dispatch_kling(self, record, &selection, &inputs).await,
            "volcengine" | "jimeng" => {
                self::surfaces::dispatch_volcengine(self, record, &selection, &inputs).await
            }
            other => Err(GenerationsError::Provider(format!(
                "image vendor {other:?} is not supported by the generations provider adapter"
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
        let vendor = vendor_from_record(record);
        match vendor.as_str() {
            "nano-banana" => {
                let task = self
                    .gateway
                    .nano_banana_retrieve_image_generation(task_id)
                    .await
                    .map_err(|error| GenerationsError::Provider(error.to_string()))?;
                Ok(Some(self::surfaces::outcome_from_provider_task(
                    record,
                    &inputs,
                    &vendor,
                    task.task_id.as_deref().or(task.id.as_deref()),
                    task.status.as_deref().or(task.state.as_deref()),
                    task.images.clone().unwrap_or_default(),
                    task.error.as_ref().map(crate::task_error_message),
                )))
            }
            "kling" => {
                let task = self
                    .gateway
                    .kling_retrieve_image_generation(task_id)
                    .await
                    .map_err(|error| GenerationsError::Provider(error.to_string()))?;
                Ok(Some(self::surfaces::outcome_from_provider_task(
                    record,
                    &inputs,
                    &vendor,
                    task.task_id.as_deref().or(task.id.as_deref()),
                    task.status.as_deref().or(task.state.as_deref()),
                    task.images.clone().unwrap_or_default(),
                    task.error.as_ref().map(crate::task_error_message),
                )))
            }
            "vidu" => {
                let task = self
                    .gateway
                    .vidu_retrieve_image_creations(task_id)
                    .await
                    .map_err(|error| GenerationsError::Provider(error.to_string()))?;
                let mut results = Vec::new();
                for creation in task.creations.iter().flatten() {
                    if let Some(url) = creation
                        .image_url
                        .clone()
                        .or_else(|| creation.url.clone())
                    {
                        results.push(image_result(record, &url, None));
                    }
                }
                let usage =
                    crate::usage::usage_from_vidu_creations(&vendor, &inputs, &task);
                Ok(Some(finish_outcome(record, results, Some(usage))))
            }
            _ => Ok(None),
        }
    }
}

/// Vendor surfaces for image generation, split out of the trait impl.
mod surfaces {
    use super::*;

    pub(super) async fn dispatch_openai(
        adapter: &ImageGenerationProviderAdapter,
        record: &GenerationRecord,
        selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let _ = selection;
        let model = model_or_default(inputs, "gpt-image-2");
        if record.operation_type == "image_edit" {
            let Some(reference) = inputs.first_reference_image() else {
                return Err(GenerationsError::InvalidInput(
                    "image_edit requires at least one reference image".to_string(),
                ));
            };
            let request = OpenAiImageEditRequest {
                image: Some(OpenAiImageReferenceInputList {
                    additional_properties: [("url".to_string(), serde_json::json!(reference))]
                        .into_iter()
                        .collect(),
                }),
                mask: None,
                model: model.clone(),
                prompt: inputs.prompt.clone(),
                ..Default::default()
            };
            let list = adapter
                .gateway
                .openai_create_image_edit(&request)
                .await
                .map_err(|error| GenerationsError::Provider(error.to_string()))?;
            Ok(finish_outcome(
                record,
                list.data
                    .iter()
                    .map(|image| image_result_from_openai(record, image))
                    .collect(),
                Some(usage_from_openai_image_list("openai", inputs, &list)),
            ))
        } else {
            let request = OpenAiImageGenerationRequest {
                model: model.clone(),
                n: inputs.image_count,
                prompt: inputs.prompt.clone(),
                quality: inputs.quality.clone(),
                response_format: inputs.response_format.clone(),
                size: inputs.size.clone(),
            };
            let list = adapter
                .gateway
                .openai_create_image_generation(&request)
                .await
                .map_err(|error| GenerationsError::Provider(error.to_string()))?;
            Ok(finish_outcome(
                record,
                list.data
                    .iter()
                    .map(|image| image_result_from_openai(record, image))
                    .collect(),
                Some(usage_from_openai_image_list("openai", inputs, &list)),
            ))
        }
    }

    pub(super) async fn dispatch_nano_banana(
        adapter: &ImageGenerationProviderAdapter,
        record: &GenerationRecord,
        _selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let request = NanoBananaImageGenerationRequest {
            aspect_ratio: inputs.aspect_ratio.clone(),
            callback_url: None,
            images: (!inputs.reference_images.is_empty()).then(|| inputs.reference_images.clone()),
            model: (!inputs.model.is_empty()).then(|| inputs.model.clone()),
            prompt: inputs.prompt.clone(),
            seed: inputs.seed,
            size: inputs.size.clone(),
        };
        let task = adapter
            .gateway
            .nano_banana_create_image_generation(&request)
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        finish_task_dispatch(record, inputs, "nano-banana", task.task_id.as_deref().or(task.id.as_deref()), task.status.as_deref().or(task.state.as_deref()), task.images.clone().unwrap_or_default(), task.error.as_ref().map(crate::task_error_message))
    }

    pub(super) async fn dispatch_vidu(
        adapter: &ImageGenerationProviderAdapter,
        record: &GenerationRecord,
        _selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        if inputs.reference_images.is_empty() {
            return Err(GenerationsError::InvalidInput(
                "vidu reference_to_image requires at least one reference image".to_string(),
            ));
        }
        let request = ViduReferenceToImageRequest {
            aspect_ratio: inputs.aspect_ratio.clone(),
            callback_url: None,
            images: inputs.reference_images.clone(),
            model: model_or_default(inputs, "viduq1-reference2image"),
            payload: None,
            prompt: inputs.prompt.clone(),
            seed: inputs.seed,
            style: None,
        };
        let task = adapter
            .gateway
            .vidu_create_reference_to_image(&request)
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        Ok(pending_outcome(
            record,
            task.task_id.as_deref().unwrap_or_default(),
            vec![task_event(record, task.task_id.as_deref().unwrap_or_default())],
        ))
    }

    pub(super) async fn dispatch_kling(
        adapter: &ImageGenerationProviderAdapter,
        record: &GenerationRecord,
        _selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let request = crate::gateway::KlingImageGenerationRequest {
            model_name: (!inputs.model.is_empty()).then(|| inputs.model.clone()),
            prompt: inputs.prompt.clone(),
            image: inputs.first_reference_image(),
            image_reference_list: None,
            aspect_ratio: inputs.aspect_ratio.clone(),
            callback_url: None,
        };
        let task = adapter
            .gateway
            .kling_create_image_generation(&request)
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        finish_task_dispatch(record, inputs, "kling", task.task_id.as_deref().or(task.id.as_deref()), task.status.as_deref().or(task.state.as_deref()), task.images.clone().unwrap_or_default(), task.error.as_ref().map(crate::task_error_message))
    }

    pub(super) async fn dispatch_volcengine(
        adapter: &ImageGenerationProviderAdapter,
        record: &GenerationRecord,
        _selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let request = VolcengineImageGenerationRequest {
            model: model_or_default(inputs, "doubao-seedream-4-0-250828"),
            prompt: inputs.prompt.clone(),
            image: (!inputs.reference_images.is_empty()).then(|| inputs.reference_images.clone()),
            size: inputs.size.clone(),
            response_format: inputs.response_format.clone(),
            seed: inputs.seed,
            watermark: None,
        };
        let response = adapter
            .gateway
            .volcengine_create_image_generation(&request)
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        let mut usage = GenerationUsage::new("volcengine");
        usage.model = Some(model_or_default(inputs, "doubao-seedream"));
        usage.image_count = response
            .data
            .iter()
            .flatten()
            .count()
            .try_into()
            .unwrap_or_default();
        if let Some(vendor_usage) = response.usage.as_ref() {
            usage.input_tokens = vendor_usage.total_tokens.unwrap_or_default();
        }
        Ok(finish_outcome(
            record,
            response
                .data
                .unwrap_or_default()
                .iter()
                .filter_map(|image| image.url.clone())
                .map(|url| image_result(record, &url, None))
                .collect(),
            Some(usage),
        ))
    }

    /// Build the dispatch outcome for a task-style image response that may
    /// already be finished.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn finish_task_dispatch(
        record: &GenerationRecord,
        inputs: &GenerationCommandInputs,
        vendor: &str,
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
            GenerationStatus::Succeeded => Ok(finish_outcome(
                record,
                media
                    .iter()
                    .filter_map(|entry| entry.url.clone().or_else(|| entry.uri.clone()))
                    .map(|url| image_result(record, &url, None))
                    .collect(),
                Some(usage_from_media(vendor, inputs, &media, MediaUsageKind::Image)),
            )),
            GenerationStatus::Failed => Err(GenerationsError::Provider(error.unwrap_or_else(
                || format!("vendor task {task_id} failed"),
            ))),
            _ => Ok(pending_outcome(record, task_id, vec![task_event(record, task_id)])),
        }
    }

    /// Build an outcome from a retrieved provider task envelope.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn outcome_from_provider_task(
        record: &GenerationRecord,
        inputs: &GenerationCommandInputs,
        vendor: &str,
        _task_id: Option<&str>,
        status: Option<&str>,
        media: Vec<ProviderGeneratedMedia>,
        error: Option<String>,
    ) -> GenerationDispatchOutcome {
        match status_from_vendor(status) {
            GenerationStatus::Succeeded => {
                let results = media
                    .iter()
                    .filter_map(|entry| entry.url.clone().or_else(|| entry.uri.clone()))
                    .map(|url| image_result(record, &url, None))
                    .collect();
                finish_outcome(
                    record,
                    results,
                    Some(usage_from_media(vendor, inputs, &media, MediaUsageKind::Image)),
                )
            }
            GenerationStatus::Failed => failed_outcome(
                record,
                &error.unwrap_or_else(|| "vendor task failed".to_string()),
            ),
            _ => record_outcome({
                let mut pending = record.clone();
                pending.status = status_from_vendor(status);
                pending
            }),
        }
    }
}

/// Shared helper assembling a succeeded outcome with results and usage.
pub(super) fn finish_outcome(
    record: &GenerationRecord,
    results: Vec<GenerationResult>,
    usage: Option<GenerationUsage>,
) -> GenerationDispatchOutcome {
    succeeded_outcome(record, results, usage, Vec::new())
}

fn image_result_from_openai(
    record: &GenerationRecord,
    image: &cloudrouter_open_sdk::models::OpenAiImage,
) -> GenerationResult {
    image_result(
        record,
        image.url.as_deref().unwrap_or_default(),
        image.mime_type.as_deref(),
    )
}

fn image_result(
    record: &GenerationRecord,
    url: &str,
    mime_type: Option<&str>,
) -> GenerationResult {
    GenerationResult {
        id: format!("{}:image-{}", record.id, uuid::Uuid::new_v4()),
        generation_id: record.id.clone(),
        result_type: "image".to_string(),
        drive_space_id: None,
        drive_node_id: None,
        drive_uri: None,
        resource_snapshot: Some(MediaResource {
            media_resource_id: None,
            kind: Some("image".to_string()),
            source: Some("generated".to_string()),
            url: Some(url.to_string()),
            public_url: Some(url.to_string()),
            uri: Some(url.to_string()),
            media_type: Some("image".to_string()),
            content_type: mime_type.map(str::to_string),
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

fn vendor_from_record(record: &GenerationRecord) -> String {
    record
        .source_provider
        .clone()
        .unwrap_or_else(|| "openai".to_string())
}
