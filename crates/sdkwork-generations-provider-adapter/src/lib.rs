//! Vendor adapters for sdkwork-generations.
//!
//! Every adapter implements [`sdkwork_intelligence_generations_service::ports::GenerationProvider`]
//! and calls vendor open APIs through the generated cloudrouter Rust SDK
//! (`cloudrouter_open_sdk::SdkworkAiClient`). Vendor surfaces that are not yet
//! bound in the generated SDK (Kling image generation, Volcengine image
//! generation) travel through the SDK's authenticated HTTP transport with
//! typed request/response models owned by this crate.

pub mod gateway;
pub mod image;
pub mod music;
pub mod registry;
pub mod sfx;
pub mod usage;
pub mod video;
pub mod vendor;
pub mod voice;

pub use gateway::{CloudRouterMediaGateway, GatewaySettings, MediaSdkGateway};
pub use image::ImageGenerationProviderAdapter;
pub use music::MusicGenerationProviderAdapter;
pub use registry::{build_providers, build_providers_from_env};
pub use sfx::SoundEffectGenerationProviderAdapter;
pub use video::VideoGenerationProviderAdapter;
pub use voice::VoiceGenerationProviderAdapter;

use sdkwork_intelligence_generations_service::domain::models::{
    GenerationRecord, GenerationResult, GenerationStatus, GenerationTimelineEvent,
};
use sdkwork_intelligence_generations_service::ports::GenerationDispatchOutcome;

/// Build a dispatch outcome for a record that only changes state.
pub(crate) fn record_outcome(record: GenerationRecord) -> GenerationDispatchOutcome {
    GenerationDispatchOutcome::from_record(record)
}

/// Build a failed dispatch outcome with a terminal timeline event.
pub(crate) fn failed_outcome(
    record: &GenerationRecord,
    message: &str,
) -> GenerationDispatchOutcome {
    let mut failed = record.clone();
    failed.status = GenerationStatus::Failed;
    GenerationDispatchOutcome {
        record: failed,
        results: Vec::new(),
        timeline_events: vec![failure_event(record, message)],
        usage: None,
    }
}

/// Build a successful dispatch outcome with normalized results and usage.
pub(crate) fn succeeded_outcome(
    record: &GenerationRecord,
    results: Vec<GenerationResult>,
    usage: Option<sdkwork_intelligence_generations_service::ports::GenerationUsage>,
    extra_events: Vec<GenerationTimelineEvent>,
) -> GenerationDispatchOutcome {
    let mut succeeded = record.clone();
    succeeded.status = GenerationStatus::Succeeded;
    succeeded.result_count = results.len() as i32;
    GenerationDispatchOutcome {
        record: succeeded,
        results,
        timeline_events: extra_events,
        usage,
    }
}

/// Build a pending (async task) dispatch outcome.
pub(crate) fn pending_outcome(
    record: &GenerationRecord,
    task_id: &str,
    events: Vec<GenerationTimelineEvent>,
) -> GenerationDispatchOutcome {
    let mut pending = record.clone();
    pending.status = GenerationStatus::Running;
    pending.source_job_id = Some(task_id.to_string());
    GenerationDispatchOutcome {
        record: pending,
        results: Vec::new(),
        timeline_events: events,
        usage: None,
    }
}

/// Timeline event describing a provider failure.
pub(crate) fn failure_event(record: &GenerationRecord, message: &str) -> GenerationTimelineEvent {
    GenerationTimelineEvent {
        id: format!("{}:provider-failed", record.id),
        generation_id: record.id.clone(),
        event_type: "generation.provider_failed".to_string(),
        message: Some(message.to_string()),
        payload: None,
        created_at: now_iso(),
    }
}

/// Timeline event describing an accepted async provider task.
pub(crate) fn task_event(record: &GenerationRecord, task_id: &str) -> GenerationTimelineEvent {
    GenerationTimelineEvent {
        id: format!("{}:provider-task", record.id),
        generation_id: record.id.clone(),
        event_type: "generation.provider_task".to_string(),
        message: Some(format!("provider task {task_id} accepted")),
        payload: Some(serde_json::json!({ "taskId": task_id })),
        created_at: now_iso(),
    }
}

/// Normalize a provider task status into the generation status enum.
pub(crate) fn status_from_vendor(status: Option<&str>) -> GenerationStatus {
    match status.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("succeeded") | Some("completed") | Some("success") | Some("complete") => {
            GenerationStatus::Succeeded
        }
        Some("failed") | Some("error") | Some("expired") => GenerationStatus::Failed,
        Some("queued") | Some("pending") | Some("submitted") => GenerationStatus::Queued,
        _ => GenerationStatus::Running,
    }
}

/// Current UTC timestamp in RFC 3339 format.
pub(crate) fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Render a vendor task error into a human-readable message.
pub(crate) fn task_error_message(error: &cloudrouter_open_sdk::models::ProviderTaskError) -> String {
    match (error.code.as_deref(), error.message.as_deref()) {
        (Some(code), Some(message)) => format!("{code}: {message}"),
        (Some(code), None) => code.to_string(),
        (None, Some(message)) => message.to_string(),
        (None, None) => "unknown vendor task error".to_string(),
    }
}
