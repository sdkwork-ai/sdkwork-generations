//! Voice provider adapters (speech synthesis, transcription, translation)
//! through the OpenAI-compatible audio surface of the media gateway.

use std::sync::Arc;

use async_trait::async_trait;
use cloudrouter_open_sdk::models::{
    OpenAiAudioTranscriptionRequest, OpenAiAudioTranslationRequest, OpenAiFileReferenceInput,
    OpenAiSpeechCreateRequest,
};
use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
use sdkwork_intelligence_generations_service::domain::models::{
    CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationResult,
    MediaResource,
};
use sdkwork_intelligence_generations_service::error::GenerationsError;
use sdkwork_intelligence_generations_service::ports::{
    GenerationDispatchOutcome, GenerationProvider, GenerationUsage,
};

use crate::gateway::MediaSdkGateway;
use crate::vendor::{resolve_vendor, GenerationCommandInputs};
use crate::{succeeded_outcome};

/// Voice generation provider dispatching through the media gateway.
pub struct VoiceGenerationProviderAdapter {
    gateway: Arc<dyn MediaSdkGateway>,
    default_vendor: String,
}

impl VoiceGenerationProviderAdapter {
    /// Create a voice provider bound to a media gateway.
    pub fn new(gateway: Arc<dyn MediaSdkGateway>, default_vendor: impl Into<String>) -> Self {
        Self {
            gateway,
            default_vendor: default_vendor.into(),
        }
    }
}

#[async_trait]
impl GenerationProvider for VoiceGenerationProviderAdapter {
    fn modality(&self) -> GenerationModality {
        GenerationModality::Voice
    }

    fn operation_types(&self) -> Vec<&str> {
        vec!["speech", "transcription", "translation"]
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
        if selection.vendor != "openai" {
            return Err(GenerationsError::Provider(format!(
                "voice vendor {:?} is not supported by the generations provider adapter",
                selection.vendor
            )));
        }
        let inputs = GenerationCommandInputs::from_command(command);
        match record.operation_type.as_str() {
            "speech" => dispatch_speech(self, record, &inputs).await,
            "transcription" => dispatch_transcription(self, record, &inputs).await,
            "translation" => dispatch_translation(self, record, &inputs).await,
            other => Err(GenerationsError::InvalidInput(format!(
                "voice operation {other:?} is not supported"
            ))),
        }
    }
}

async fn dispatch_speech(
    adapter: &VoiceGenerationProviderAdapter,
    record: &GenerationRecord,
    inputs: &GenerationCommandInputs,
) -> Result<GenerationDispatchOutcome, GenerationsError> {
    let request = OpenAiSpeechCreateRequest {
        input: inputs.prompt.clone(),
        metadata: None,
        model: model_or_default(inputs, "gpt-4o-mini-tts"),
        response_format: inputs.response_format.clone(),
        speed: inputs.speed,
        voice: inputs.voice.clone().unwrap_or_else(|| "alloy".to_string()),
    };
    let bytes = adapter
        .gateway
        .openai_create_speech(&request)
        .await
        .map_err(|error| GenerationsError::Provider(error.to_string()))?;
    if bytes.is_empty() {
        return Err(GenerationsError::Provider(
            "speech synthesis returned no audio bytes".to_string(),
        ));
    }
    use base64::Engine as _;
    let url = format!(
        "data:audio/mpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    );
    let mut usage = GenerationUsage::new("openai");
    usage.model = Some(model_or_default(inputs, "gpt-4o-mini-tts"));
    Ok(succeeded_outcome(
        record,
        vec![audio_result(record, &url, "speech", None)],
        Some(usage),
        Vec::new(),
    ))
}

async fn dispatch_transcription(
    adapter: &VoiceGenerationProviderAdapter,
    record: &GenerationRecord,
    inputs: &GenerationCommandInputs,
) -> Result<GenerationDispatchOutcome, GenerationsError> {
    let reference = audio_reference(inputs);
    let request = OpenAiAudioTranscriptionRequest {
        file: OpenAiFileReferenceInput {
            additional_properties: [("file".to_string(), serde_json::json!(reference))]
                .into_iter()
                .collect(),
        },
        language: None,
        model: model_or_default(inputs, "whisper-1"),
        prompt: Some(inputs.prompt.clone()).filter(|value| !value.is_empty()),
        response_format: inputs.response_format.clone(),
    };
    let transcription = adapter
        .gateway
        .openai_create_transcription(&request)
        .await
        .map_err(|error| GenerationsError::Provider(error.to_string()))?;
    let result = GenerationResult {
        id: format!("{}:transcription-{}", record.id, uuid::Uuid::new_v4()),
        generation_id: record.id.clone(),
        result_type: "text".to_string(),
        drive_space_id: None,
        drive_node_id: None,
        drive_uri: None,
        resource_snapshot: None,
        asset_id: None,
        preview_text: Some(transcription.text.clone()),
        created_at: crate::now_iso(),
    };
    let mut usage = GenerationUsage::new("openai");
    usage.audio_seconds = transcription.duration.unwrap_or_default();
    usage.model = Some(model_or_default(inputs, "whisper-1"));
    Ok(succeeded_outcome(record, vec![result], Some(usage), Vec::new()))
}

async fn dispatch_translation(
    adapter: &VoiceGenerationProviderAdapter,
    record: &GenerationRecord,
    inputs: &GenerationCommandInputs,
) -> Result<GenerationDispatchOutcome, GenerationsError> {
    let reference = audio_reference(inputs);
    let request = OpenAiAudioTranslationRequest {
        file: OpenAiFileReferenceInput {
            additional_properties: [("file".to_string(), serde_json::json!(reference))]
                .into_iter()
                .collect(),
        },
        model: model_or_default(inputs, "whisper-1"),
        prompt: Some(inputs.prompt.clone()).filter(|value| !value.is_empty()),
        response_format: inputs.response_format.clone(),
    };
    let translation = adapter
        .gateway
        .openai_create_translation(&request)
        .await
        .map_err(|error| GenerationsError::Provider(error.to_string()))?;
    let result = GenerationResult {
        id: format!("{}:translation-{}", record.id, uuid::Uuid::new_v4()),
        generation_id: record.id.clone(),
        result_type: "text".to_string(),
        drive_space_id: None,
        drive_node_id: None,
        drive_uri: None,
        resource_snapshot: None,
        asset_id: None,
        preview_text: Some(translation.text.clone()),
        created_at: crate::now_iso(),
    };
    let mut usage = GenerationUsage::new("openai");
    usage.audio_seconds = translation.duration.unwrap_or_default();
    usage.model = Some(model_or_default(inputs, "whisper-1"));
    Ok(succeeded_outcome(record, vec![result], Some(usage), Vec::new()))
}

fn audio_reference(inputs: &GenerationCommandInputs) -> String {
    inputs
        .first_reference_image()
        .or_else(|| inputs.input_asset_ids.first().cloned())
        .unwrap_or_default()
}

fn audio_result(
    record: &GenerationRecord,
    url: &str,
    media_type: &str,
    duration_seconds: Option<f64>,
) -> GenerationResult {
    GenerationResult {
        id: format!("{}:audio-{}", record.id, uuid::Uuid::new_v4()),
        generation_id: record.id.clone(),
        result_type: "audio".to_string(),
        drive_space_id: None,
        drive_node_id: None,
        drive_uri: None,
        resource_snapshot: Some(MediaResource {
            media_resource_id: None,
            kind: Some("audio".to_string()),
            source: Some("generated".to_string()),
            url: Some(url.to_string()),
            public_url: Some(url.to_string()),
            uri: Some(url.to_string()),
            media_type: Some(media_type.to_string()),
            content_type: Some("audio/mpeg".to_string()),
            width: None,
            height: None,
            duration_ms: duration_seconds.map(|seconds| (seconds * 1000.0) as i64),
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
