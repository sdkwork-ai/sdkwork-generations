//! Music generation vendor adapters (Suno through the media gateway).

use std::sync::Arc;

use async_trait::async_trait;
use cloudrouter_open_sdk::models::SunoMusicGenerationRequest;
use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
use sdkwork_intelligence_generations_service::domain::models::{
    CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationResult,
    GenerationStatus, MediaResource,
};
use sdkwork_intelligence_generations_service::error::GenerationsError;
use sdkwork_intelligence_generations_service::ports::{GenerationDispatchOutcome, GenerationProvider};

use crate::gateway::MediaSdkGateway;
use crate::usage::usage_from_suno_task;
use crate::vendor::{resolve_vendor, GenerationCommandInputs};
use crate::{failed_outcome, pending_outcome, record_outcome, status_from_vendor, succeeded_outcome, task_event};

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
        if selection.vendor != "suno" {
            return Err(GenerationsError::Provider(format!(
                "music vendor {:?} is not supported by the generations provider adapter",
                selection.vendor
            )));
        }
        let inputs = GenerationCommandInputs::from_command(command);
        let prompt = if record.operation_type == "lyrics_to_music" {
            compose_lyrics_prompt(&inputs)
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
        let response = self
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

fn compose_lyrics_prompt(inputs: &GenerationCommandInputs) -> String {
    let mut segments = Vec::new();
    if let Some(lyrics) = inputs.lyrics.as_deref().filter(|value| !value.trim().is_empty()) {
        segments.push(lyrics.to_string());
    }
    segments.push(inputs.prompt.clone());
    segments.join("\n\n")
}

fn music_result(record: &GenerationRecord, url: &str, duration_seconds: Option<f64>) -> GenerationResult {
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
