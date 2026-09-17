//! Voice provider adapters (speech synthesis, transcription, translation).
//!
//! Speech synthesis is vendor-unified like image generation: one adapter
//! dispatching to per-vendor surfaces — OpenAI-compatible audio speech
//! (`gpt-4o-mini-tts`), ElevenLabs text-to-speech (`voice_id` path surface),
//! and Volcengine Ark speech (`/api/v3/audio/speech`, OpenAI-shaped body).
//! Transcription and translation stay on the OpenAI-compatible surface.

use std::sync::Arc;

use async_trait::async_trait;
use cloudrouter_open_sdk::models::{
    ElevenLabsTextToSpeechRequest, OpenAiAudioTranscriptionRequest, OpenAiAudioTranslationRequest,
    OpenAiFileReferenceInput, OpenAiSpeechCreateRequest,
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
use crate::vendor::{resolve_vendor, GenerationCommandInputs, VendorSelection};
use crate::{succeeded_outcome, with_resolved_vendor};

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
        let inputs = GenerationCommandInputs::from_command(command, &selection);
        // Persist the resolved vendor so reads/reporting attribute the record
        // to the surface that dispatched it (mirrors the image adapter).
        let record = with_resolved_vendor(record, &selection.vendor);
        match record.operation_type.as_str() {
            "speech" => match selection.vendor.as_str() {
                "openai" => self::surfaces::dispatch_speech_openai(self, &record, &inputs).await,
                "elevenlabs" => {
                    self::surfaces::dispatch_speech_elevenlabs(self, &record, &selection, &inputs)
                        .await
                }
                "volcengine" | "jimeng" => {
                    self::surfaces::dispatch_speech_volcengine(self, &record, &selection, &inputs)
                        .await
                }
                other => Err(GenerationsError::Provider(format!(
                    "voice vendor {other:?} is not supported by the generations provider adapter"
                ))),
            },
            "transcription" => dispatch_transcription(self, &record, &inputs).await,
            "translation" => dispatch_translation(self, &record, &inputs).await,
            other => Err(GenerationsError::InvalidInput(format!(
                "voice operation {other:?} is not supported"
            ))),
        }
    }
}

/// Vendor surfaces for speech synthesis, split out of the trait impl.
mod surfaces {
    use super::*;

    pub(super) async fn dispatch_speech_openai(
        adapter: &VoiceGenerationProviderAdapter,
        record: &GenerationRecord,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let model = model_or_default(inputs, "gpt-4o-mini-tts");
        let request = OpenAiSpeechCreateRequest {
            input: inputs.prompt.clone(),
            metadata: None,
            model: model.clone(),
            response_format: inputs.response_format.clone(),
            speed: inputs.speed,
            voice: inputs.voice.clone().unwrap_or_else(|| "alloy".to_string()),
        };
        let bytes = speech_bytes(
            adapter
                .gateway
                .openai_create_speech(&request)
                .await,
        )?;
        let mime = audio_mime_from_format(inputs.response_format.as_deref());
        let mut usage = GenerationUsage::new("openai");
        usage.model = Some(model);
        Ok(succeeded_outcome(
            record,
            vec![audio_result(record, &data_url(&bytes, mime), mime, None)],
            Some(usage),
            Vec::new(),
        ))
    }

    pub(super) async fn dispatch_speech_elevenlabs(
        adapter: &VoiceGenerationProviderAdapter,
        record: &GenerationRecord,
        selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let voice_id = inputs
            .voice_id
            .clone()
            .or_else(|| inputs.voice.clone())
            .ok_or_else(|| {
                GenerationsError::InvalidInput(
                    "elevenlabs speech requires a voiceId (ElevenLabs voice id)".to_string(),
                )
            })?;
        let model = model_or_default(inputs, "eleven_multilingual_v2");
        let request = ElevenLabsTextToSpeechRequest {
            model_id: model.clone(),
            text: inputs.prompt.clone(),
            voice_settings: voice_settings(inputs),
        };
        let response = adapter
            .gateway
            .elevenlabs_create_text_to_speech(&voice_id, &request, inputs.response_format.as_deref())
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        let url = response
            .audio_url
            .clone()
            .or_else(|| response.url.clone())
            .ok_or_else(|| {
                GenerationsError::Provider(format!(
                    "elevenlabs speech response is missing the audio url (status {:?}, id {:?})",
                    response.status, response.id
                ))
            })?;
        let mime = audio_mime_from_format(inputs.response_format.as_deref());
        let mut usage = GenerationUsage::new("elevenlabs");
        usage.model = Some(model);
        let _ = selection;
        Ok(succeeded_outcome(
            record,
            vec![audio_result(record, &url, mime, None)],
            Some(usage),
            Vec::new(),
        ))
    }

    pub(super) async fn dispatch_speech_volcengine(
        adapter: &VoiceGenerationProviderAdapter,
        record: &GenerationRecord,
        selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let model = model_or_default(inputs, "doubao-tts");
        let request = OpenAiSpeechCreateRequest {
            input: inputs.prompt.clone(),
            metadata: None,
            model: model.clone(),
            // Ark speech accepts the OpenAI response_format/speed fields; the
            // speed ratio mirrors the shared speech speed parameter.
            response_format: inputs.response_format.clone(),
            speed: inputs.speed,
            voice: inputs
                .voice
                .clone()
                .or_else(|| inputs.voice_id.clone())
                .unwrap_or_else(|| "zh_female_cancan_mars".to_string()),
        };
        let bytes = speech_bytes(adapter.gateway.volcengine_create_speech(&request).await)?;
        let mime = audio_mime_from_format(inputs.response_format.as_deref());
        let mut usage = GenerationUsage::new("volcengine");
        usage.model = Some(model);
        let _ = selection;
        Ok(succeeded_outcome(
            record,
            vec![audio_result(record, &data_url(&bytes, mime), mime, None)],
            Some(usage),
            Vec::new(),
        ))
    }
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
        language: inputs.language.clone(),
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

/// Build ElevenLabs `voice_settings` from the shared speech parameters.
fn voice_settings(inputs: &GenerationCommandInputs) -> Option<serde_json::Value> {
    let mut settings = serde_json::Map::new();
    if let Some(stability) = inputs.stability {
        settings.insert("stability".to_string(), serde_json::json!(stability));
    }
    if let Some(similarity) = inputs.similarity {
        settings.insert(
            "similarity_boost".to_string(),
            serde_json::json!(similarity),
        );
    }
    if let Some(style) = inputs.style {
        settings.insert("style".to_string(), serde_json::json!(style));
    }
    if let Some(speed) = inputs.speed {
        settings.insert("speed".to_string(), serde_json::json!(speed));
    }
    (!settings.is_empty()).then_some(serde_json::Value::Object(settings))
}

fn speech_bytes(result: Result<Vec<u8>, cloudrouter_open_sdk::SdkworkError>) -> Result<Vec<u8>, GenerationsError> {
    let bytes = result.map_err(|error| GenerationsError::Provider(error.to_string()))?;
    if bytes.is_empty() {
        return Err(GenerationsError::Provider(
            "speech synthesis returned no audio bytes".to_string(),
        ));
    }
    Ok(bytes)
}

fn data_url(bytes: &[u8], mime: &str) -> String {
    use base64::Engine as _;
    format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

/// Map a requested response format onto the audio content type.
fn audio_mime_from_format(response_format: Option<&str>) -> &'static str {
    match response_format.map(str::trim).map(str::to_ascii_lowercase) {
        Some(format) => match format.as_str() {
            "wav" => "audio/wav",
            "opus" | "ogg" => "audio/ogg",
            "aac" => "audio/aac",
            "flac" => "audio/flac",
            "pcm" => "audio/pcm",
            // ElevenLabs formats like `mp3_44100_128` still carry the mp3
            // container; everything unrecognized defaults to mp3.
            _ => "audio/mpeg",
        },
        None => "audio/mpeg",
    }
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
            content_type: Some(media_type.to_string()),
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

    use cloudrouter_open_sdk::models::ElevenLabsTextToSpeechResponse;
    use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
    use sdkwork_intelligence_generations_service::domain::models::{
        CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationStatus,
    };
    use sdkwork_intelligence_generations_service::ports::{GenerationDispatchOutcome, GenerationProvider};

    use crate::gateway::test_support::ScriptedGateway;
    use crate::vendor::VENDOR_ELEVENLABS;

    use super::*;

    fn sample_record() -> GenerationRecord {
        GenerationRecord {
            id: "gen-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            user_id: "user-1".to_string(),
            modality: GenerationModality::Voice,
            operation_type: "speech".to_string(),
            source_provider: None,
            source_job_id: None,
            prompt_preview: Some("hello world".to_string()),
            status: GenerationStatus::Queued,
            favorite: false,
            result_count: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn speech_command(
        vendor: &str,
        model: &str,
        parameters: serde_json::Value,
    ) -> CreateGenerationCommandRequest {
        let mut parameters = parameters;
        if let Some(object) = parameters.as_object_mut() {
            object.insert("vendor".to_string(), serde_json::json!(vendor));
        } else {
            parameters = serde_json::json!({ "vendor": vendor });
        }
        CreateGenerationCommandRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "hello world".to_string(),
            model: Some(model.to_string()),
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

    async fn dispatch(
        gateway: Arc<ScriptedGateway>,
        command: &CreateGenerationCommandRequest,
    ) -> GenerationDispatchOutcome {
        let adapter = VoiceGenerationProviderAdapter::new(gateway, "openai");
        adapter
            .dispatch(&sample_record(), command, &context())
            .await
            .expect("speech dispatch must succeed")
    }

    #[tokio::test]
    async fn speech_openai_uses_openai_surface_with_model_and_voice() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.openai_speech.lock().unwrap() = Some(vec![9u8, 8, 7]);
        let command = speech_command(
            "openai",
            "gpt-4o-mini-tts",
            serde_json::json!({ "voice": "alloy", "responseFormat": "mp3" }),
        );
        let outcome = dispatch(gateway.clone(), &command).await;
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.record.source_provider.as_deref(), Some("openai"));
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.content_type.as_deref(), Some("audio/mpeg"));
        assert!(snapshot.url.as_deref().unwrap().starts_with("data:audio/mpeg;base64,"));
        let request = gateway
            .last_openai_speech_request
            .lock()
            .unwrap()
            .clone()
            .expect("openai speech request captured");
        assert_eq!(request.model, "gpt-4o-mini-tts");
        assert_eq!(request.input, "hello world");
        assert_eq!(request.voice, "alloy");
    }

    #[tokio::test]
    async fn speech_elevenlabs_uses_voice_id_surface() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.elevenlabs_text_to_speech.lock().unwrap() = Some(ElevenLabsTextToSpeechResponse {
            audio_url: Some("https://cdn.example/speech.mp3".to_string()),
            id: Some("tts-1".to_string()),
            status: Some("succeeded".to_string()),
            url: None,
        });
        let command = speech_command(
            VENDOR_ELEVENLABS,
            "eleven_multilingual_v2",
            serde_json::json!({
                "voiceId": "21m00Tcm4TlvDq8ikWAM",
                "stability": 0.4,
                "similarity": 0.8,
            }),
        );
        let outcome = dispatch(gateway.clone(), &command).await;
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.record.source_provider.as_deref(), Some(VENDOR_ELEVENLABS));
        assert_eq!(outcome.results.len(), 1);
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.url.as_deref(), Some("https://cdn.example/speech.mp3"));
        assert_eq!(snapshot.content_type.as_deref(), Some("audio/mpeg"));
        let (voice_id, request, _format) = gateway
            .last_elevenlabs_text_to_speech
            .lock()
            .unwrap()
            .clone()
            .expect("elevenlabs speech request captured");
        assert_eq!(voice_id, "21m00Tcm4TlvDq8ikWAM");
        assert_eq!(request.model_id, "eleven_multilingual_v2");
        assert_eq!(request.text, "hello world");
        let settings = request.voice_settings.expect("voice settings mapped");
        assert_eq!(settings["stability"], serde_json::json!(0.4));
        assert_eq!(settings["similarity_boost"], serde_json::json!(0.8));
    }

    #[tokio::test]
    async fn speech_elevenlabs_similarity_only_still_builds_voice_settings() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.elevenlabs_text_to_speech.lock().unwrap() = Some(ElevenLabsTextToSpeechResponse {
            audio_url: Some("https://cdn.example/speech.mp3".to_string()),
            ..Default::default()
        });
        let command = speech_command(
            VENDOR_ELEVENLABS,
            "eleven_multilingual_v2",
            serde_json::json!({
                "voiceId": "21m00Tcm4TlvDq8ikWAM",
                "similarity": 0.9
            }),
        );
        let _ = dispatch(gateway.clone(), &command).await;
        let (_voice_id, request, _format) = gateway
            .last_elevenlabs_text_to_speech
            .lock()
            .unwrap()
            .clone()
            .expect("elevenlabs speech request captured");
        let settings = request.voice_settings.expect("voice settings built from similarity alone");
        assert_eq!(settings["similarity_boost"], serde_json::json!(0.9));
        assert!(settings.get("stability").is_none());
    }

    #[tokio::test]
    async fn speech_elevenlabs_requires_a_voice_id() {
        let gateway = Arc::new(ScriptedGateway::default());
        let adapter = VoiceGenerationProviderAdapter::new(gateway, "openai");
        let command = speech_command(VENDOR_ELEVENLABS, "eleven_multilingual_v2", serde_json::json!({}));
        let error = adapter
            .dispatch(&sample_record(), &command, &context())
            .await
            .expect_err("missing voice id must be rejected");
        assert!(error.to_string().contains("voiceId"));
    }

    #[tokio::test]
    async fn speech_volcengine_uses_ark_surface_and_wav_mime() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.volcengine_speech.lock().unwrap() = Some(vec![1u8, 2, 3, 4]);
        let command = speech_command(
            "volcengine",
            "doubao-tts",
            serde_json::json!({
                "voice": "zh_female_cancan_mars",
                "responseFormat": "wav",
            }),
        );
        let outcome = dispatch(gateway.clone(), &command).await;
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.record.source_provider.as_deref(), Some("volcengine"));
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.content_type.as_deref(), Some("audio/wav"));
        assert!(snapshot.url.as_deref().unwrap().starts_with("data:audio/wav;base64,"));
        let request = gateway
            .last_volcengine_speech_request
            .lock()
            .unwrap()
            .clone()
            .expect("volcengine speech request captured");
        assert_eq!(request.model, "doubao-tts");
        assert_eq!(request.input, "hello world");
        assert_eq!(request.voice, "zh_female_cancan_mars");
        assert_eq!(request.response_format.as_deref(), Some("wav"));
    }

    #[tokio::test]
    async fn speech_rejects_unknown_vendor() {
        let gateway = Arc::new(ScriptedGateway::default());
        let adapter = VoiceGenerationProviderAdapter::new(gateway, "openai");
        let command = speech_command("suno", "suno-tts", serde_json::json!({}));
        let error = adapter
            .dispatch(&sample_record(), &command, &context())
            .await
            .expect_err("unknown speech vendor must be rejected");
        assert!(error.to_string().contains("not supported"));
    }
}
