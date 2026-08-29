//! Sound effect provider adapter (ElevenLabs through the media gateway).

use std::sync::Arc;

use async_trait::async_trait;
use cloudrouter_open_sdk::models::ElevenLabsSoundGenerationRequest;
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
use crate::succeeded_outcome;

/// Default ElevenLabs sound-effect model id.
const DEFAULT_SOUND_EFFECT_MODEL: &str = "eleven_audio_v2";
/// Default ElevenLabs output format for generated sound effects.
const DEFAULT_SOUND_OUTPUT_FORMAT: &str = "mp3_44100_128";

/// Sound effect generation provider dispatching through the media gateway.
pub struct SoundEffectGenerationProviderAdapter {
    gateway: Arc<dyn MediaSdkGateway>,
    default_vendor: String,
}

impl SoundEffectGenerationProviderAdapter {
    /// Create a sound effect provider bound to a media gateway.
    pub fn new(gateway: Arc<dyn MediaSdkGateway>, default_vendor: impl Into<String>) -> Self {
        Self {
            gateway,
            default_vendor: default_vendor.into(),
        }
    }
}

#[async_trait]
impl GenerationProvider for SoundEffectGenerationProviderAdapter {
    fn modality(&self) -> GenerationModality {
        GenerationModality::Sfx
    }

    fn operation_types(&self) -> Vec<&str> {
        vec!["sound_effects"]
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
        if selection.vendor != "elevenlabs" {
            return Err(GenerationsError::Provider(format!(
                "sound effect vendor {:?} is not supported by the generations provider adapter",
                selection.vendor
            )));
        }
        let inputs = GenerationCommandInputs::from_command(command);
        let request = ElevenLabsSoundGenerationRequest {
            model_id: model_or_default(&inputs, DEFAULT_SOUND_EFFECT_MODEL),
            text: inputs.prompt.clone(),
            duration_seconds: inputs.duration_seconds,
            prompt_influence: inputs.cfg_scale,
            r#loop: inputs.loop_enabled,
        };
        let response = self
            .gateway
            .elevenlabs_create_sound_generation(&request, Some(DEFAULT_SOUND_OUTPUT_FORMAT))
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;

        let url = response
            .audio_url
            .clone()
            .or_else(|| response.url.clone())
            .or_else(|| {
                response
                    .audio
                    .as_ref()
                    .and_then(|audio| audio.get("url"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            });
        let Some(url) = url.filter(|value| !value.trim().is_empty()) else {
            return Err(GenerationsError::Provider(
                "elevenlabs sound generation returned no audio location".to_string(),
            ));
        };

        let mut usage = GenerationUsage::new("elevenlabs");
        usage.model = Some(model_or_default(&inputs, DEFAULT_SOUND_EFFECT_MODEL));
        usage.audio_seconds = inputs.duration_seconds.unwrap_or_default();

        Ok(succeeded_outcome(
            record,
            vec![sound_effect_result(record, &url, inputs.duration_seconds)],
            Some(usage),
            Vec::new(),
        ))
    }
}

fn sound_effect_result(
    record: &GenerationRecord,
    url: &str,
    duration_seconds: Option<f64>,
) -> GenerationResult {
    GenerationResult {
        id: format!("{}:sfx-{}", record.id, uuid::Uuid::new_v4()),
        generation_id: record.id.clone(),
        result_type: "sound_effect".to_string(),
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
            media_type: Some("sfx".to_string()),
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
