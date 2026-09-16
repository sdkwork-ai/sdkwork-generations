//! Sound effect provider adapter.
//!
//! Sound effects are vendor-unified like image generation: one adapter
//! dispatching to per-vendor surfaces. Today the ElevenLabs sound-generation
//! surface is the only vendor; additional vendors slot in as new surfaces
//! under `dispatch` instead of new adapters.

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
use crate::vendor::{resolve_vendor, GenerationCommandInputs, VendorSelection};
use crate::{succeeded_outcome, with_resolved_vendor};

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
        let inputs = GenerationCommandInputs::from_command(command, &selection);
        // Persist the resolved vendor for read-path attribution (mirrors the
        // image/music/voice adapters).
        let record = with_resolved_vendor(record, &selection.vendor);
        match selection.vendor.as_str() {
            "elevenlabs" => {
                self::surfaces::dispatch_elevenlabs(self, &record, &selection, &inputs).await
            }
            other => Err(GenerationsError::Provider(format!(
                "sound effect vendor {other:?} is not supported by the generations provider adapter"
            ))),
        }
    }
}

/// Vendor surfaces for sound effect generation, split out of the trait impl.
mod surfaces {
    use super::*;

    pub(super) async fn dispatch_elevenlabs(
        adapter: &SoundEffectGenerationProviderAdapter,
        record: &GenerationRecord,
        _selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let output_format = inputs
            .response_format
            .clone()
            .unwrap_or_else(|| DEFAULT_SOUND_OUTPUT_FORMAT.to_string());
        let request = ElevenLabsSoundGenerationRequest {
            model_id: model_or_default(inputs, DEFAULT_SOUND_EFFECT_MODEL),
            text: inputs.prompt.clone(),
            duration_seconds: inputs.duration_seconds,
            prompt_influence: inputs.cfg_scale,
            r#loop: inputs.loop_enabled,
        };
        let response = adapter
            .gateway
            .elevenlabs_create_sound_generation(&request, Some(&output_format))
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

        let mime = audio_mime_from_output_format(Some(output_format.as_str()));
        let mut usage = GenerationUsage::new("elevenlabs");
        usage.model = Some(model_or_default(inputs, DEFAULT_SOUND_EFFECT_MODEL));
        usage.audio_seconds = inputs.duration_seconds.unwrap_or_default();

        Ok(succeeded_outcome(
            record,
            vec![sound_effect_result(record, &url, mime, inputs.duration_seconds)],
            Some(usage),
            Vec::new(),
        ))
    }
}

/// Map an ElevenLabs output format (`mp3_44100_128`, `wav_44100`, ...) onto
/// the audio content type.
fn audio_mime_from_output_format(output_format: Option<&str>) -> &'static str {
    match output_format
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some(format) if format.starts_with("wav") => "audio/wav",
        Some(format) if format.starts_with("ogg") => "audio/ogg",
        Some(format) if format.starts_with("aac") => "audio/aac",
        Some(format) if format.starts_with("flac") => "audio/flac",
        Some(format) if format.starts_with("pcm") => "audio/pcm",
        // mp3_* and anything unrecognized default to the mp3 container.
        _ => "audio/mpeg",
    }
}

fn sound_effect_result(
    record: &GenerationRecord,
    url: &str,
    mime: &str,
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
            content_type: Some(mime.to_string()),
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cloudrouter_open_sdk::models::ElevenLabsSoundGenerationResponse;
    use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
    use sdkwork_intelligence_generations_service::domain::models::{
        CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationStatus,
    };
    use sdkwork_intelligence_generations_service::ports::GenerationProvider;

    use crate::gateway::test_support::ScriptedGateway;

    use super::*;

    fn sample_record() -> GenerationRecord {
        GenerationRecord {
            id: "gen-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            user_id: "user-1".to_string(),
            modality: GenerationModality::Sfx,
            operation_type: "sound_effects".to_string(),
            source_provider: None,
            source_job_id: None,
            prompt_preview: Some("thunder roll".to_string()),
            status: GenerationStatus::Queued,
            favorite: false,
            result_count: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn sfx_command(vendor: &str, parameters: serde_json::Value) -> CreateGenerationCommandRequest {
        let mut parameters = parameters;
        if let Some(object) = parameters.as_object_mut() {
            object.insert("vendor".to_string(), serde_json::json!(vendor));
        } else {
            parameters = serde_json::json!({ "vendor": vendor });
        }
        CreateGenerationCommandRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "thunder roll".to_string(),
            model: None,
            input_asset_ids: None,
            parameters: Some(parameters),
        }
    }

    fn context() -> GenerationsRequestContext {
        GenerationsRequestContext::from_parts(
            "tenant-1".to_string(),
            "user-1".to_string(),
            "trace-1".to_string(),
        )
    }

    #[tokio::test]
    async fn sfx_elevenlabs_dispatch_maps_request_and_default_mime() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.elevenlabs_sound_generation.lock().unwrap() =
            Some(ElevenLabsSoundGenerationResponse {
                audio_url: Some("https://cdn.example/thunder.mp3".to_string()),
                ..Default::default()
            });
        let adapter = SoundEffectGenerationProviderAdapter::new(gateway.clone(), "elevenlabs");
        let outcome = adapter
            .dispatch(
                &sample_record(),
                &sfx_command(
                    "elevenlabs",
                    serde_json::json!({"durationSeconds": 3.5, "promptInfluence": 0.6}),
                ),
                &context(),
            )
            .await
            .expect("sfx dispatch must succeed");
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.record.source_provider.as_deref(), Some("elevenlabs"));
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.content_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(snapshot.duration_ms, Some(3500));
        assert_eq!(outcome.usage.as_ref().unwrap().audio_seconds, 3.5);
    }

    #[tokio::test]
    async fn sfx_elevenlabs_wav_output_format_maps_wav_mime() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.elevenlabs_sound_generation.lock().unwrap() =
            Some(ElevenLabsSoundGenerationResponse {
                audio_url: Some("https://cdn.example/thunder.wav".to_string()),
                ..Default::default()
            });
        let adapter = SoundEffectGenerationProviderAdapter::new(gateway.clone(), "elevenlabs");
        let outcome = adapter
            .dispatch(
                &sample_record(),
                &sfx_command(
                    "elevenlabs",
                    serde_json::json!({"responseFormat": "wav_44100"}),
                ),
                &context(),
            )
            .await
            .expect("sfx dispatch must succeed");
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.content_type.as_deref(), Some("audio/wav"));
    }

    #[tokio::test]
    async fn sfx_rejects_unknown_vendor() {
        let gateway = Arc::new(ScriptedGateway::default());
        let adapter = SoundEffectGenerationProviderAdapter::new(gateway, "elevenlabs");
        let error = adapter
            .dispatch(
                &sample_record(),
                &sfx_command("openai", serde_json::json!({})),
                &context(),
            )
            .await
            .expect_err("unknown sfx vendor must be rejected");
        assert!(error.to_string().contains("not supported"));
    }
}
