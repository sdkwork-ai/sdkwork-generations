//! Image generation vendor adapters.
//!
//! Supported vendor surfaces: OpenAI-compatible image generations/edits
//! (including `gpt-image-2`), Google nano-banana task API, Vidu reference to
//! image, Kling image generation, and Volcengine Ark image generation.

use std::sync::Arc;

use async_trait::async_trait;
use cloudrouter_open_sdk::models::{
    OpenAiImageEditRequest, OpenAiImageGenerationRequest,
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

use crate::gateway::{
    GeminiImageGenerationInstance, GeminiImageGenerationParameters, GeminiImageGenerationRequest,
    MediaSdkGateway, VolcengineImageGenerationRequest,
};
use crate::{
    failed_outcome, pending_outcome, record_outcome, status_from_vendor, succeeded_outcome,
    task_event, with_resolved_vendor,
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
        let inputs = GenerationCommandInputs::from_command(command, &selection);
        // The refresh path routes polling by record.source_provider; persist
        // the resolved vendor so the same surface that dispatched the task
        // also polls it, even when it differs from the adapter default.
        let record = with_resolved_vendor(record, &selection.vendor);
        match selection.vendor.as_str() {
            "openai" | "midjourney" => {
                self::surfaces::dispatch_openai(self, &record, &selection, &inputs).await
            }
            "nano-banana" => self::surfaces::dispatch_nano_banana(self, &record, &selection, &inputs).await,
            "vidu" => self::surfaces::dispatch_vidu(self, &record, &selection, &inputs).await,
            "kling" => self::surfaces::dispatch_kling(self, &record, &selection, &inputs).await,
            "volcengine" | "jimeng" => {
                self::surfaces::dispatch_volcengine(self, &record, &selection, &inputs).await
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
            // The Gemini image face (`:generateImages`) is synchronous: a
            // dispatched command either already carries its predictions or
            // failed, so no vendor task id is ever persisted for it and the
            // refresh path has nothing to poll.
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
                    task.model.as_deref(),
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
        // The wired Gemini image ingress is the vendor-native
        // `/google/v1beta/models/{model}:generateImages` face (the
        // `/nano-banana/...` aggregator face is contract-published but the
        // routing contract declares it unrouted, so the gateway answers it
        // fail-closed). The surface is synchronous: predictions come back on
        // the create response instead of a vendor task id.
        //
        // Gemini instances accept base64-encoded reference bytes only, while
        // the command plane carries image URLs — reference-conditioned
        // commands cannot be translated faithfully and fail honestly instead
        // of silently dropping the references.
        if !inputs.reference_images.is_empty() {
            return Err(GenerationsError::InvalidInput(
                "gemini image generation does not accept reference image URLs; supply base64-encoded image bytes".to_string(),
            ));
        }
        let parameters = GeminiImageGenerationParameters {
            sample_count: inputs.image_count,
            aspect_ratio: inputs.aspect_ratio.clone(),
            person_generation: None,
        };
        let request = GeminiImageGenerationRequest {
            instances: vec![GeminiImageGenerationInstance {
                prompt: inputs.prompt.clone(),
                image: None,
            }],
            parameters: Some(parameters),
        };
        let response = adapter
            .gateway
            .gemini_create_image_generation(&inputs.model, &request)
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        if let Some(error) = response.error.as_ref() {
            return Err(GenerationsError::Provider(crate::task_error_message(
                error,
            )));
        }
        let locators = response.image_locators();
        if locators.is_empty() {
            return Err(GenerationsError::Provider(
                "gemini image generation finished without media".to_string(),
            ));
        }
        let media = locators
            .iter()
            .map(|(locator, _)| ProviderGeneratedMedia {
                url: Some(locator.clone()),
                ..ProviderGeneratedMedia::default()
            })
            .collect::<Vec<_>>();
        let results = locators
            .iter()
            .map(|(locator, mime_type)| image_result(record, locator, mime_type.as_deref()))
            .collect::<Vec<_>>();
        let mut usage = crate::usage::usage_from_media(
            "google",
            inputs,
            &media,
            crate::usage::MediaUsageKind::Image,
        );
        usage.model = Some(inputs.model.clone()).filter(|value| !value.is_empty());
        Ok(finish_outcome(record, results, Some(usage)))
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
        // Kling's two reference modes are mutually exclusive: a single
        // reference rides `image` (image-to-image), while multi-reference
        // models take `image_reference_list`. Mixing both is ambiguous
        // upstream, and forwarding only the first reference silently dropped
        // every additional one.
        let references: Vec<&str> = inputs
            .reference_images
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .collect();
        let request = crate::gateway::KlingImageGenerationRequest {
            model_name: (!inputs.model.is_empty()).then(|| inputs.model.clone()),
            prompt: inputs.prompt.clone(),
            image: (references.len() == 1).then(|| references[0].to_string()),
            image_reference_list: (references.len() > 1).then(|| {
                references
                    .iter()
                    .map(|url| serde_json::json!({ "url": url }))
                    .collect()
            }),
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
        vendor_model: Option<&str>,
    ) -> GenerationDispatchOutcome {
        match status_from_vendor(status) {
            GenerationStatus::Succeeded => {
                let results = media
                    .iter()
                    .filter_map(|entry| entry.url.clone().or_else(|| entry.uri.clone()))
                    .map(|url| image_result(record, &url, None))
                    .collect();
                let mut usage =
                    usage_from_media(vendor, inputs, &media, MediaUsageKind::Image);
                // The refresh path carries default inputs, so the
                // vendor-reported model is the only metering model source.
                if let Some(model) =
                    vendor_model.map(str::trim).filter(|model| !model.is_empty())
                {
                    usage.model = Some(model.to_string());
                }
                finish_outcome(record, results, Some(usage))
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
    // gpt-image models answer base64-only (no hosted `url`); surface the
    // payload as a data URI so the result keeps a renderable locator instead
    // of an empty one.
    let url = image
        .url
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| openai_b64_data_uri(image));
    image_result(record, url.as_deref().unwrap_or_default(), image.mime_type.as_deref())
}

fn openai_b64_data_uri(image: &cloudrouter_open_sdk::models::OpenAiImage) -> Option<String> {
    let payload = image.b64_json.as_deref()?.trim();
    (!payload.is_empty()).then(|| {
        format!(
            "data:{};base64,{payload}",
            image.mime_type.as_deref().unwrap_or("image/png")
        )
    })
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cloudrouter_open_sdk::models::OpenAiImage;
    use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
    use sdkwork_intelligence_generations_service::domain::models::{
        CreateGenerationCommandRequest, GenerationModality, GenerationStatus,
    };
    use sdkwork_intelligence_generations_service::ports::GenerationProvider;

    use crate::gateway::test_support::ScriptedGateway;
    use crate::image::ImageGenerationProviderAdapter;

    fn sample_record(
        operation_type: &str,
    ) -> sdkwork_intelligence_generations_service::domain::models::GenerationRecord {
        sdkwork_intelligence_generations_service::domain::models::GenerationRecord {
            id: "gen-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            user_id: "user-1".to_string(),
            modality: GenerationModality::Image,
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

    fn command(
        model: &str,
        parameters: Option<serde_json::Value>,
    ) -> CreateGenerationCommandRequest {
        CreateGenerationCommandRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "a precise router diagram".to_string(),
            model: Some(model.to_string()),
            input_asset_ids: None,
            parameters,
        }
    }

    fn context() -> GenerationsRequestContext {
        GenerationsRequestContext::from_parts(
            "tenant-1".to_string(),
            "user-1".to_string(),
            "trace-1".to_string(),
        )
    }

    fn reference_parameters(entries: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "vendor": "openai",
            "referenceImages": entries,
            "generationConfig": {
                "aspectRatio": "1:1",
                "imageCount": 2,
                "size": "1024x1024",
                "quality": "high",
                "responseFormat": "url"
            }
        })
    }

    #[tokio::test]
    async fn openai_text_to_image_forwards_size_quality_count_and_format() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.openai_image_generation.lock().unwrap() = Some(cloudrouter_open_sdk::models::OpenAiImageList {
            created: 0,
            data: vec![OpenAiImage {
                url: Some("https://cdn.example/out.png".to_string()),
                ..OpenAiImage::default()
            }],
            usage: None,
        });
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        let outcome = provider
            .dispatch(
                &sample_record("text_to_image"),
                &command("openai/gpt-image-2", Some(reference_parameters(serde_json::json!([])))),
                &context(),
            )
            .await
            .expect("openai dispatch succeeds");

        let request = gateway
            .last_openai_image_request
            .lock()
            .unwrap()
            .clone()
            .expect("openai request captured");
        assert_eq!(request.model, "gpt-image-2", "the vendor prefix is stripped");
        assert_eq!(request.n, Some(2));
        assert_eq!(request.size.as_deref(), Some("1024x1024"));
        assert_eq!(request.quality.as_deref(), Some("high"));
        assert_eq!(request.response_format.as_deref(), Some("url"));
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(
            outcome.results[0].resource_snapshot.as_ref().unwrap().url.as_deref(),
            Some("https://cdn.example/out.png")
        );
    }

    #[tokio::test]
    async fn openai_b64_only_response_becomes_a_data_uri() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.openai_image_generation.lock().unwrap() = Some(cloudrouter_open_sdk::models::OpenAiImageList {
            created: 0,
            data: vec![OpenAiImage {
                b64_json: Some("aW1hZ2VieXRlcw==".to_string()),
                ..OpenAiImage::default()
            }],
            usage: None,
        });
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        let outcome = provider
            .dispatch(
                &sample_record("text_to_image"),
                &command("openai/gpt-image-2", None),
                &context(),
            )
            .await
            .expect("openai dispatch succeeds");

        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(
            snapshot.url.as_deref(),
            Some("data:image/png;base64,aW1hZ2VieXRlcw=="),
            "base64-only gpt-image output stays renderable"
        );
    }

    #[tokio::test]
    async fn openai_image_edit_forwards_the_reference_url() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.openai_image_edit_list.lock().unwrap() = Some(cloudrouter_open_sdk::models::OpenAiImageList {
            created: 0,
            data: vec![OpenAiImage {
                url: Some("https://cdn.example/edited.png".to_string()),
                ..OpenAiImage::default()
            }],
            usage: None,
        });
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        provider
            .dispatch(
                &sample_record("image_edit"),
                &command(
                    "openai/gpt-image-2",
                    Some(reference_parameters(serde_json::json!([
                        { "url": "https://cdn.example/ref.png" }
                    ]))),
                ),
                &context(),
            )
            .await
            .expect("openai edit dispatch succeeds");

        let request = gateway
            .last_openai_image_edit_request
            .lock()
            .unwrap()
            .clone()
            .expect("openai edit request captured");
        assert_eq!(request.prompt, "a precise router diagram");
        let reference = request.image.expect("reference rides the edit request");
        assert_eq!(
            reference
                .additional_properties
                .get("url")
                .and_then(serde_json::Value::as_str),
            Some("https://cdn.example/ref.png")
        );
    }

    #[tokio::test]
    async fn kling_single_reference_rides_the_image_field() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.kling_image_create_task.lock().unwrap() =
            Some(crate::gateway::ProviderImageGenerationTask {
                task_id: Some("kling-task-1".to_string()),
                ..crate::gateway::ProviderImageGenerationTask::default()
            });
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        let mut parameters = reference_parameters(serde_json::json!([
            { "url": "https://cdn.example/ref.png" }
        ]));
        parameters["vendor"] = serde_json::json!("kling");
        let outcome = provider
            .dispatch(
                &sample_record("text_to_image"),
                &command("kling/kling-v2-6", Some(parameters)),
                &context(),
            )
            .await
            .expect("kling dispatch succeeds");

        let request = gateway
            .last_kling_image_request
            .lock()
            .unwrap()
            .clone()
            .expect("kling request captured");
        assert_eq!(request.model_name.as_deref(), Some("kling-v2-6"));
        assert_eq!(request.image.as_deref(), Some("https://cdn.example/ref.png"));
        assert!(request.image_reference_list.is_none(), "single reference stays on image");
        assert_eq!(request.aspect_ratio.as_deref(), Some("1:1"));
        assert_eq!(outcome.record.status, GenerationStatus::Running);
        assert_eq!(outcome.record.source_job_id.as_deref(), Some("kling-task-1"));
        assert_eq!(outcome.record.source_provider.as_deref(), Some("kling"));
    }

    #[tokio::test]
    async fn kling_multi_reference_forwards_the_full_reference_list() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.kling_image_create_task.lock().unwrap() =
            Some(crate::gateway::ProviderImageGenerationTask {
                task_id: Some("kling-task-2".to_string()),
                ..crate::gateway::ProviderImageGenerationTask::default()
            });
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        let mut parameters = reference_parameters(serde_json::json!([
            { "url": "https://cdn.example/ref-1.png" },
            { "url": "https://cdn.example/ref-2.png" },
            { "url": "https://cdn.example/ref-3.png" }
        ]));
        parameters["vendor"] = serde_json::json!("kling");
        provider
            .dispatch(
                &sample_record("text_to_image"),
                &command("kling/kling-v2-master", Some(parameters)),
                &context(),
            )
            .await
            .expect("kling dispatch succeeds");

        let request = gateway
            .last_kling_image_request
            .lock()
            .unwrap()
            .clone()
            .expect("kling request captured");
        assert!(
            request.image.is_none(),
            "multi-reference does not set the single image field"
        );
        let list = request
            .image_reference_list
            .expect("multi-reference list is forwarded");
        assert_eq!(list.len(), 3, "every reference is forwarded");
        assert_eq!(
            list[0].get("url").and_then(serde_json::Value::as_str),
            Some("https://cdn.example/ref-1.png")
        );
    }

    #[tokio::test]
    async fn kling_retrieve_collects_succeeded_images() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.kling_image_retrieve_task.lock().unwrap() =
            Some(crate::gateway::ProviderImageGenerationTask {
                task_id: Some("kling-task-1".to_string()),
                status: Some("succeed".to_string()),
                model: Some("kling-image-v2".to_string()),
                images: Some(vec![cloudrouter_open_sdk::models::ProviderGeneratedMedia {
                    url: Some("https://cdn.example/kling.png".to_string()),
                    ..cloudrouter_open_sdk::models::ProviderGeneratedMedia::default()
                }]),
                ..crate::gateway::ProviderImageGenerationTask::default()
            });
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        let mut record = sample_record("text_to_image");
        record.source_provider = Some("kling".to_string());
        record.source_job_id = Some("kling-task-1".to_string());
        let outcome = provider
            .retrieve(&record, &context())
            .await
            .expect("kling retrieve succeeds")
            .expect("kling retrieve produces an outcome");

        assert_eq!(
            gateway.last_kling_image_retrieve_task_id.lock().unwrap().as_deref(),
            Some("kling-task-1")
        );
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(
            outcome.results[0].resource_snapshot.as_ref().unwrap().url.as_deref(),
            Some("https://cdn.example/kling.png")
        );
        // The refresh path holds no original command, so the vendor-reported
        // model is the metering model.
        let usage = outcome.usage.expect("usage is reported");
        assert_eq!(usage.model.as_deref(), Some("kling-image-v2"));
        assert_eq!(usage.image_count, 1);
    }

    #[tokio::test]
    async fn vidu_reference_to_image_forwards_every_reference_and_pends() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.vidu_image_task.lock().unwrap() =
            Some(serde_json::from_str(r#"{"task_id": "vidu-task-1"}"#).expect("vidu task fixture"));
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        let mut parameters = reference_parameters(serde_json::json!([
            { "url": "https://cdn.example/ref-1.png" },
            { "url": "https://cdn.example/ref-2.png" }
        ]));
        parameters["vendor"] = serde_json::json!("vidu");
        parameters["seed"] = serde_json::json!(42);
        let outcome = provider
            .dispatch(
                &sample_record("text_to_image"),
                &command("vidu/viduq1-reference2image", Some(parameters)),
                &context(),
            )
            .await
            .expect("vidu dispatch succeeds");

        let request = gateway
            .last_vidu_image_request
            .lock()
            .unwrap()
            .clone()
            .expect("vidu request captured");
        assert_eq!(request.model, "viduq1-reference2image");
        assert_eq!(
            request.images,
            vec![
                "https://cdn.example/ref-1.png".to_string(),
                "https://cdn.example/ref-2.png".to_string()
            ]
        );
        assert_eq!(request.aspect_ratio.as_deref(), Some("1:1"));
        assert_eq!(request.seed, Some(42));
        assert_eq!(outcome.record.status, GenerationStatus::Running);
        assert_eq!(outcome.record.source_job_id.as_deref(), Some("vidu-task-1"));
    }

    #[tokio::test]
    async fn volcengine_forwards_size_seed_references_and_counts_usage() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.volcengine_image_response.lock().unwrap() = Some(
            serde_json::from_str(
                r#"{
                    "data": [ { "url": "https://cdn.example/seedream.png" } ],
                    "usage": { "generated_images": 1, "total_tokens": 1200 }
                }"#,
            )
            .expect("volcengine response fixture"),
        );
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        let mut parameters = reference_parameters(serde_json::json!([
            { "url": "https://cdn.example/ref.png" }
        ]));
        parameters["vendor"] = serde_json::json!("jimeng");
        let outcome = provider
            .dispatch(
                &sample_record("text_to_image"),
                &command("jimeng/doubao-seedream-4-0-250828", Some(parameters)),
                &context(),
            )
            .await
            .expect("volcengine dispatch succeeds");

        let request = gateway
            .last_volcengine_image_request
            .lock()
            .unwrap()
            .clone()
            .expect("volcengine request captured");
        assert_eq!(request.model, "doubao-seedream-4-0-250828");
        assert_eq!(
            request.image,
            Some(vec!["https://cdn.example/ref.png".to_string()])
        );
        assert_eq!(request.size.as_deref(), Some("1024x1024"));
        assert_eq!(request.response_format.as_deref(), Some("url"));
        assert_eq!(request.seed, None, "no seed supplied by the fixture");
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(
            outcome.results[0].resource_snapshot.as_ref().unwrap().url.as_deref(),
            Some("https://cdn.example/seedream.png")
        );
        let usage = outcome.usage.expect("volcengine usage is reported");
        assert_eq!(usage.image_count, 1);
        assert_eq!(usage.input_tokens, 1200);
    }

    #[tokio::test]
    async fn nano_banana_forwards_sample_count_and_aspect_ratio() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.gemini_image_generation.lock().unwrap() =
            Some(crate::gateway::GeminiImageGenerationResponse {
                predictions: vec![crate::gateway::GeminiImagePrediction {
                    url: Some("https://cdn.example/gemini.png".to_string()),
                    ..crate::gateway::GeminiImagePrediction::default()
                }],
                error: None,
            });
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        let mut parameters = reference_parameters(serde_json::json!([]));
        parameters["vendor"] = serde_json::json!("google");
        let outcome = provider
            .dispatch(
                &sample_record("text_to_image"),
                &command("google/gemini-2.5-flash-image", Some(parameters)),
                &context(),
            )
            .await
            .expect("gemini dispatch succeeds");

        let (model, request) = gateway
            .last_gemini_image_request
            .lock()
            .unwrap()
            .clone()
            .expect("gemini request captured");
        assert_eq!(model, "gemini-2.5-flash-image");
        let parameters = request.parameters.expect("gemini parameters forwarded");
        assert_eq!(parameters.sample_count, Some(2));
        assert_eq!(parameters.aspect_ratio.as_deref(), Some("1:1"));
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(
            outcome.results[0].resource_snapshot.as_ref().unwrap().url.as_deref(),
            Some("https://cdn.example/gemini.png")
        );
    }
}

