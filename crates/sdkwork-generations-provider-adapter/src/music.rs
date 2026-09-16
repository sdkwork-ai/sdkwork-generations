//! Music generation vendor adapters.
//!
//! Music is vendor-unified like image generation: one adapter dispatching to
//! per-vendor surfaces — Suno task-style music generation (create + polled
//! retrieve) and MiniMax synchronous music generation (`/v1/music_generation`
//! with url output).

use std::sync::Arc;

use async_trait::async_trait;
use cloudrouter_open_sdk::models::{MiniMaxMusicGenerationRequest, SunoMusicGenerationRequest};
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
use crate::usage::usage_from_suno_task;
use crate::vendor::{resolve_vendor, GenerationCommandInputs, VendorSelection};
use crate::{
    failed_outcome, pending_outcome, record_outcome, status_from_vendor, succeeded_outcome,
    task_event, with_resolved_vendor,
};

/// Music generation provider dispatching through the media gateway.
pub struct MusicGenerationProviderAdapter {
    gateway: Arc<dyn MediaSdkGateway>,
    default_vendor: String,
}

impl MusicGenerationProviderAdapter {
    /// Create a music provider bound to a media gateway.
    pub fn new(gateway: Arc<dyn MediaSdkGateway>, default_vendor: impl Into<String>) -> Self {
        Self {
            gateway,
            default_vendor: default_vendor.into(),
        }
    }
}

#[async_trait]
impl GenerationProvider for MusicGenerationProviderAdapter {
    fn modality(&self) -> GenerationModality {
        GenerationModality::Music
    }

    fn operation_types(&self) -> Vec<&str> {
        vec!["text_to_music", "lyrics_to_music"]
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
        // Persist the resolved vendor so the retrieve path polls with the
        // surface that created the task.
        let record = with_resolved_vendor(record, &selection.vendor);
        match selection.vendor.as_str() {
            "suno" => self::surfaces::dispatch_suno(self, &record, &selection, &inputs).await,
            "minimax" => self::surfaces::dispatch_minimax(self, &record, &selection, &inputs).await,
            other => Err(GenerationsError::Provider(format!(
                "music vendor {other:?} is not supported by the generations provider adapter"
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
        if record.source_provider.as_deref() != Some("suno") {
            return Ok(None);
        }
        let inputs = GenerationCommandInputs::default();
        let task = self
            .gateway
            .suno_retrieve_music_generation(task_id)
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        let status = status_from_vendor(task.status.as_deref());
        match status {
            GenerationStatus::Succeeded => {
                let results = task
                    .tracks
                    .iter()
                    .flatten()
                    .filter_map(|track| {
                        track
                            .audio_url
                            .clone()
                            .map(|url| music_result(record, &url, track.duration))
                    })
                    .collect();
                let usage = usage_from_suno_task("suno", &inputs, &task);
                Ok(Some(succeeded_outcome(record, results, Some(usage), Vec::new())))
            }
            GenerationStatus::Failed => Ok(Some(failed_outcome(record, "suno music task failed"))),
            _ => Ok(Some(record_outcome({
                let mut running = record.clone();
                running.status = status;
                running
            }))),
        }
    }
}

/// Vendor surfaces for music generation, split out of the trait impl.
mod surfaces {
    use super::*;

    pub(super) async fn dispatch_suno(
        adapter: &MusicGenerationProviderAdapter,
        record: &GenerationRecord,
        _selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let prompt = if record.operation_type == "lyrics_to_music" {
            compose_lyrics_prompt(inputs)
        } else {
            inputs.prompt.clone()
        };
        let request = SunoMusicGenerationRequest {
            callback_url: None,
            duration: inputs.duration_seconds,
            model: (!inputs.model.is_empty()).then(|| inputs.model.clone()),
            negative_tags: inputs.negative_tags.clone(),
            prompt,
            tags: inputs.tags.clone(),
            title: inputs.title.clone(),
        };
        let response = adapter
            .gateway
            .suno_create_music_generation(&request)
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        let Some(task_id) = response
            .task_id
            .as_deref()
            .or(response.id.as_deref())
            .filter(|value| !value.trim().is_empty())
        else {
            return Err(GenerationsError::Provider(
                "suno task response is missing task_id".to_string(),
            ));
        };
        if status_from_vendor(response.status.as_deref()) == GenerationStatus::Failed {
            return Err(GenerationsError::Provider(format!(
                "suno task {task_id} failed"
            )));
        }
        Ok(pending_outcome(record, task_id, vec![task_event(record, task_id)]))
    }

    pub(super) async fn dispatch_minimax(
        adapter: &MusicGenerationProviderAdapter,
        record: &GenerationRecord,
        _selection: &VendorSelection,
        inputs: &GenerationCommandInputs,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let model = if inputs.model.trim().is_empty() {
            "music-3.0".to_string()
        } else {
            inputs.model.clone()
        };
        let request = MiniMaxMusicGenerationRequest {
            audio_setting: build_minimax_audio_setting(inputs),
            is_instrumental: inputs.is_instrumental,
            lyrics: minimax_lyrics(record, inputs),
            lyrics_optimizer: inputs.lyrics_optimizer,
            model: model.clone(),
            // URL output keeps the result a playable link instead of a hex
            // payload that would have to be decoded and re-hosted.
            output_format: Some("url".to_string()),
            prompt: (!inputs.prompt.is_empty()).then(|| inputs.prompt.clone()),
            stream: None,
        };
        let response = adapter
            .gateway
            .minimax_create_music_generation(&request)
            .await
            .map_err(|error| GenerationsError::Provider(error.to_string()))?;
        if let Some(base) = response.base_resp.as_ref() {
            if let Some(code) = base.status_code {
                if code != 0 {
                    return Err(GenerationsError::Provider(format!(
                        "minimax music generation failed with status {code}: {}",
                        base.status_msg.as_deref().unwrap_or("no message")
                    )));
                }
            }
        }
        // MiniMax data.status: 1 means in progress, 2 means finished. The
        // synchronous surface has no polling handle, so anything unfinished
        // is surfaced as a provider error instead of a stuck pending record.
        let Some(data) = response.data.as_ref() else {
            return Err(GenerationsError::Provider(
                "minimax music generation response is missing data".to_string(),
            ));
        };
        if data.status != Some(2) {
            return Err(GenerationsError::Provider(format!(
                "minimax music generation is unfinished (data.status {:?})",
                data.status
            )));
        }
        let Some(audio) = data
            .audio
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return Err(GenerationsError::Provider(
                "minimax music generation returned no audio location".to_string(),
            ));
        };
        let duration_seconds = data
            .extra_info
            .as_ref()
            .and_then(|extra| extra.music_duration);
        let mime = audio_mime_from_format(inputs.audio_format.as_deref());
        let mut usage = GenerationUsage::new("minimax");
        usage.model = Some(model);
        usage.audio_seconds = duration_seconds.unwrap_or_default();
        Ok(succeeded_outcome(
            record,
            vec![music_result_with_mime(record, audio, mime, duration_seconds)],
            Some(usage),
            Vec::new(),
        ))
    }
}

fn compose_lyrics_prompt(inputs: &GenerationCommandInputs) -> String {
    let mut segments = Vec::new();
    if let Some(lyrics) = inputs.lyrics.as_deref().filter(|value| !value.trim().is_empty()) {
        segments.push(lyrics.to_string());
    }
    segments.push(inputs.prompt.clone());
    segments.join("\n\n")
}

/// MiniMax takes lyrics as a dedicated field, so lyrics_to_music maps directly
/// instead of composing a combined prompt like Suno.
fn minimax_lyrics(record: &GenerationRecord, inputs: &GenerationCommandInputs) -> Option<String> {
    if record.operation_type != "lyrics_to_music" {
        return None;
    }
    inputs
        .lyrics
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| (!inputs.prompt.is_empty()).then(|| inputs.prompt.clone()))
}

fn build_minimax_audio_setting(
    inputs: &GenerationCommandInputs,
) -> Option<cloudrouter_open_sdk::models::MiniMaxMusicAudioSetting> {
    if inputs.sample_rate.is_none()
        && inputs.audio_bitrate.is_none()
        && inputs.audio_format.is_none()
    {
        return None;
    }
    Some(cloudrouter_open_sdk::models::MiniMaxMusicAudioSetting {
        sample_rate: inputs.sample_rate,
        bitrate: inputs.audio_bitrate,
        format: inputs.audio_format.clone(),
    })
}

/// Map a requested audio container onto the audio content type.
fn audio_mime_from_format(format: Option<&str>) -> &'static str {
    match format.map(str::trim).map(str::to_ascii_lowercase) {
        Some(format) => match format.as_str() {
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "aac" => "audio/aac",
            "flac" => "audio/flac",
            "pcm" => "audio/pcm",
            // Anything unrecognized defaults to the mp3 container.
            _ => "audio/mpeg",
        },
        None => "audio/mpeg",
    }
}

fn music_result(
    record: &GenerationRecord,
    url: &str,
    duration_seconds: Option<f64>,
) -> GenerationResult {
    music_result_with_mime(record, url, "audio/mpeg", duration_seconds)
}

fn music_result_with_mime(
    record: &GenerationRecord,
    url: &str,
    mime: &str,
    duration_seconds: Option<f64>,
) -> GenerationResult {
    GenerationResult {
        id: format!("{}:music-{}", record.id, uuid::Uuid::new_v4()),
        generation_id: record.id.clone(),
        result_type: "music".to_string(),
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
            media_type: Some("music".to_string()),
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cloudrouter_open_sdk::models::{
        MiniMaxMusicData, MiniMaxMusicExtraInfo, MiniMaxMusicGenerationResponse,
        SunoMusicGenerationResponse, SunoMusicGenerationTaskResponse, SunoMusicTrack,
    };
    use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
    use sdkwork_intelligence_generations_service::domain::models::{
        CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationStatus,
    };
    use sdkwork_intelligence_generations_service::ports::{
        GenerationDispatchOutcome, GenerationProvider,
    };

    use crate::gateway::test_support::ScriptedGateway;

    use super::*;

    fn sample_record(operation_type: &str) -> GenerationRecord {
        GenerationRecord {
            id: "gen-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            user_id: "user-1".to_string(),
            modality: GenerationModality::Music,
            operation_type: operation_type.to_string(),
            source_provider: None,
            source_job_id: None,
            prompt_preview: Some("gen".to_string()),
            status: GenerationStatus::Queued,
            favorite: false,
            result_count: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn music_command(
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
            prompt: "rainy night synth".to_string(),
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
        record: &GenerationRecord,
        command: &CreateGenerationCommandRequest,
    ) -> GenerationDispatchOutcome {
        let adapter = MusicGenerationProviderAdapter::new(gateway, "suno");
        adapter
            .dispatch(record, command, &context())
            .await
            .expect("music dispatch must succeed")
    }

    #[tokio::test]
    async fn music_suno_dispatch_pends_on_task_id() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.suno_create_task.lock().unwrap() = Some(SunoMusicGenerationResponse {
            task_id: Some("suno-task-1".to_string()),
            status: Some("pending".to_string()),
            ..Default::default()
        });
        let command = music_command(
            "suno",
            "",
            serde_json::json!({"tags": "synth,dance", "title": "Neon Drive"}),
        );
        let outcome = dispatch(gateway, &sample_record("text_to_music"), &command).await;
        assert_eq!(outcome.record.status, GenerationStatus::Running);
        assert_eq!(outcome.record.source_provider.as_deref(), Some("suno"));
        assert_eq!(outcome.record.source_job_id.as_deref(), Some("suno-task-1"));
    }

    #[tokio::test]
    async fn music_suno_lyrics_to_music_composes_prompt() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.suno_create_task.lock().unwrap() = Some(SunoMusicGenerationResponse {
            task_id: Some("suno-task-2".to_string()),
            status: Some("pending".to_string()),
            ..Default::default()
        });
        let command = music_command(
            "suno",
            "",
            serde_json::json!({"lyrics": "[Verse]\nmoonlight"}),
        );
        let _ = dispatch(gateway.clone(), &sample_record("lyrics_to_music"), &command).await;
        let request = gateway
            .last_suno_create_request
            .lock()
            .unwrap()
            .clone()
            .expect("suno request captured");
        assert!(request.prompt.contains("[Verse]"));
        assert!(request.prompt.contains("rainy night synth"));
    }

    #[tokio::test]
    async fn music_minimax_dispatch_returns_url_result_with_wav_mime() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.minimax_music_generation.lock().unwrap() = Some(MiniMaxMusicGenerationResponse {
            data: Some(MiniMaxMusicData {
                status: Some(2),
                audio: Some("https://cdn.example/song.wav".to_string()),
                extra_info: Some(MiniMaxMusicExtraInfo {
                    music_duration: Some(42.5),
                    ..Default::default()
                }),
            }),
            ..Default::default()
        });
        let command = music_command(
            "minimax",
            "music-3.0",
            serde_json::json!({
                "lyrics": "[Verse]\nmoonlight",
                "audioFormat": "wav",
                "sampleRate": 44100,
                "isInstrumental": false
            }),
        );
        let outcome = dispatch(gateway.clone(), &sample_record("lyrics_to_music"), &command).await;
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.record.source_provider.as_deref(), Some("minimax"));
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.url.as_deref(), Some("https://cdn.example/song.wav"));
        assert_eq!(snapshot.content_type.as_deref(), Some("audio/wav"));
        assert_eq!(snapshot.duration_ms, Some(42500));
        let usage = outcome.usage.as_ref().unwrap();
        assert_eq!(usage.audio_seconds, 42.5);
        let request = gateway
            .last_minimax_music_request
            .lock()
            .unwrap()
            .clone()
            .expect("minimax request captured");
        assert_eq!(request.model, "music-3.0");
        assert_eq!(request.output_format.as_deref(), Some("url"));
        assert_eq!(request.lyrics.as_deref(), Some("[Verse]\nmoonlight"));
        let setting = request.audio_setting.as_ref().unwrap();
        assert_eq!(setting.sample_rate, Some(44100));
        assert_eq!(setting.format.as_deref(), Some("wav"));
        assert_eq!(request.is_instrumental, Some(false));
    }

    #[tokio::test]
    async fn music_minimax_surfaces_base_resp_failure() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.minimax_music_generation.lock().unwrap() = Some(MiniMaxMusicGenerationResponse {
            base_resp: Some(cloudrouter_open_sdk::models::MiniMaxMusicBaseResp {
                status_code: Some(1004),
                status_msg: Some("invalid api key".to_string()),
            }),
            ..Default::default()
        });
        let adapter = MusicGenerationProviderAdapter::new(gateway, "suno");
        let command = music_command("minimax", "music-3.0", serde_json::json!({}));
        let error = adapter
            .dispatch(&sample_record("text_to_music"), &command, &context())
            .await
            .expect_err("minimax base_resp failure must surface");
        assert!(error.to_string().contains("1004"));
    }

    #[tokio::test]
    async fn music_rejects_unknown_vendor() {
        let gateway = Arc::new(ScriptedGateway::default());
        let adapter = MusicGenerationProviderAdapter::new(gateway, "suno");
        let command = music_command("openai", "gpt-music", serde_json::json!({}));
        let error = adapter
            .dispatch(&sample_record("text_to_music"), &command, &context())
            .await
            .expect_err("unknown music vendor must be rejected");
        assert!(error.to_string().contains("not supported"));
    }

    #[tokio::test]
    async fn music_suno_retrieve_collects_tracks() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.suno_retrieve_task.lock().unwrap() = Some(SunoMusicGenerationTaskResponse {
            status: Some("succeeded".to_string()),
            tracks: Some(vec![SunoMusicTrack {
                audio_url: Some("https://cdn.example/track.mp3".to_string()),
                duration: Some(30.0),
                ..Default::default()
            }]),
            ..Default::default()
        });
        let adapter = MusicGenerationProviderAdapter::new(gateway, "suno");
        let mut record = sample_record("text_to_music");
        record.source_provider = Some("suno".to_string());
        record.source_job_id = Some("suno-task-1".to_string());
        let outcome = adapter
            .retrieve(&record, &context())
            .await
            .expect("retrieve must succeed")
            .expect("suno retrieve returns an outcome");
        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.results.len(), 1);
        assert_eq!(
            outcome.results[0]
                .resource_snapshot
                .as_ref()
                .unwrap()
                .url
                .as_deref(),
            Some("https://cdn.example/track.mp3")
        );
    }
}
