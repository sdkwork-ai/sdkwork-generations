//! Metering usage extraction from vendor responses.

use cloudrouter_open_sdk::models::{
    OpenAiImageList, ProviderGeneratedMedia, SunoMusicGenerationTaskResponse, ViduTaskCreationsResponse,
};
use sdkwork_intelligence_generations_service::ports::GenerationUsage;

use crate::vendor::GenerationCommandInputs;

/// Build a usage fact for an OpenAI-compatible image response.
pub fn usage_from_openai_image_list(
    vendor: &str,
    inputs: &GenerationCommandInputs,
    list: &OpenAiImageList,
) -> GenerationUsage {
    let mut usage = GenerationUsage::new(vendor);
    usage.model = Some(inputs.model.clone()).filter(|value| !value.is_empty());
    usage.image_count = list.data.len() as i64;
    if let Some(token_usage) = list.usage.as_ref() {
        usage.input_tokens = token_usage.prompt_tokens;
        usage.output_tokens = token_usage.completion_tokens;
    }
    usage
}

/// Build a usage fact from generated media entries (videos or audio tracks).
pub fn usage_from_media(
    vendor: &str,
    inputs: &GenerationCommandInputs,
    media: &[ProviderGeneratedMedia],
    kind: MediaUsageKind,
) -> GenerationUsage {
    let mut usage = GenerationUsage::new(vendor);
    usage.model = Some(inputs.model.clone()).filter(|value| !value.is_empty());
    for entry in media {
        let seconds = entry.duration.unwrap_or_default();
        match kind {
            MediaUsageKind::Video => usage.video_seconds += seconds,
            MediaUsageKind::Audio => usage.audio_seconds += seconds,
            MediaUsageKind::Image => usage.image_count += 1,
        }
    }
    usage
}

/// Build a usage fact from a Vidu task creations response.
pub fn usage_from_vidu_creations(
    vendor: &str,
    inputs: &GenerationCommandInputs,
    creations: &ViduTaskCreationsResponse,
) -> GenerationUsage {
    let mut usage = GenerationUsage::new(vendor);
    usage.model = Some(inputs.model.clone()).filter(|value| !value.is_empty());
    for creation in creations.creations.iter().flatten() {
        let seconds = creation.duration.unwrap_or_default();
        if creation.video_url.is_some() {
            usage.video_seconds += seconds;
        } else if creation.image_url.is_some() {
            usage.image_count += 1;
        }
    }
    usage
}

/// Build a usage fact from a Suno music task response.
pub fn usage_from_suno_task(
    vendor: &str,
    inputs: &GenerationCommandInputs,
    task: &SunoMusicGenerationTaskResponse,
) -> GenerationUsage {
    let mut usage = GenerationUsage::new(vendor);
    usage.model = Some(inputs.model.clone()).filter(|value| !value.is_empty());
    for track in task.tracks.iter().flatten() {
        usage.audio_seconds += track.duration.unwrap_or_default();
    }
    usage
}

/// Media kind used for usage accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaUsageKind {
    Image,
    Video,
    Audio,
}
