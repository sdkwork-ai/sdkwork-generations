//! Provider registry builders for the generations assembly.

use std::sync::Arc;

use sdkwork_intelligence_generations_service::ports::GenerationProvider;

use crate::gateway::{CloudRouterMediaGateway, GatewaySettings, MediaSdkGateway};
use crate::{
    image::ImageGenerationProviderAdapter, music::MusicGenerationProviderAdapter,
    sfx::SoundEffectGenerationProviderAdapter, video::VideoGenerationProviderAdapter,
    voice::VoiceGenerationProviderAdapter,
};

/// Default vendor for each modality, configurable through the environment.
pub struct ModalityDefaultVendors {
    pub image: String,
    pub video: String,
    pub music: String,
    pub voice: String,
    pub sfx: String,
}

impl Default for ModalityDefaultVendors {
    fn default() -> Self {
        Self {
            image: "openai".to_string(),
            video: "openai".to_string(),
            music: "suno".to_string(),
            voice: "openai".to_string(),
            sfx: "elevenlabs".to_string(),
        }
    }
}

impl ModalityDefaultVendors {
    /// Load per-modality default vendors from the environment.
    pub fn from_env() -> Self {
        Self {
            image: env_vendor("GENERATIONS_IMAGE_DEFAULT_VENDOR", &Self::default().image),
            video: env_vendor("GENERATIONS_VIDEO_DEFAULT_VENDOR", &Self::default().video),
            music: env_vendor("GENERATIONS_MUSIC_DEFAULT_VENDOR", &Self::default().music),
            voice: env_vendor("GENERATIONS_VOICE_DEFAULT_VENDOR", &Self::default().voice),
            sfx: env_vendor("GENERATIONS_SFX_DEFAULT_VENDOR", &Self::default().sfx),
        }
    }
}

fn env_vendor(key: &str, fallback: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

/// Build the full provider set against the environment-configured gateway.
pub fn build_providers_from_env() -> Result<Arc<Vec<Box<dyn GenerationProvider>>>, String> {
    build_providers(GatewaySettings::from_env(), ModalityDefaultVendors::from_env())
}

/// Build the full provider set against explicit settings.
pub fn build_providers(
    settings: GatewaySettings,
    vendors: ModalityDefaultVendors,
) -> Result<Arc<Vec<Box<dyn GenerationProvider>>>, String> {
    let gateway: Arc<dyn MediaSdkGateway> = Arc::new(
        CloudRouterMediaGateway::new(&settings).map_err(|error| error.to_string())?,
    );
    Ok(build_providers_for_gateway(gateway, vendors))
}

/// Build the full provider set against a caller-supplied gateway seam.
pub fn build_providers_for_gateway(
    gateway: Arc<dyn MediaSdkGateway>,
    vendors: ModalityDefaultVendors,
) -> Arc<Vec<Box<dyn GenerationProvider>>> {
    Arc::new(vec![
        Box::new(ImageGenerationProviderAdapter::new(
            Arc::clone(&gateway),
            vendors.image,
        )),
        Box::new(VideoGenerationProviderAdapter::new(
            Arc::clone(&gateway),
            vendors.video,
        )),
        Box::new(MusicGenerationProviderAdapter::new(
            Arc::clone(&gateway),
            vendors.music,
        )),
        Box::new(VoiceGenerationProviderAdapter::new(
            Arc::clone(&gateway),
            vendors.voice,
        )),
        Box::new(SoundEffectGenerationProviderAdapter::new(
            gateway,
            vendors.sfx,
        )),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_one_provider_per_modality() {
        let providers = build_providers_for_gateway(
            Arc::new(crate::gateway::test_support::ScriptedGateway::default()),
            ModalityDefaultVendors::default(),
        );
        assert_eq!(providers.len(), 5);
        let modalities: Vec<String> = providers
            .iter()
            .map(|provider| provider.modality().to_string())
            .collect();
        assert_eq!(modalities, vec!["image", "video", "music", "voice", "sfx"]);
    }
}

#[cfg(test)]
mod dispatch_tests {
    use std::sync::Arc;

    use cloudrouter_open_sdk::models::{OpenAiImage, OpenAiImageList};
    use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
    use sdkwork_intelligence_generations_service::domain::models::{
        CreateGenerationCommandRequest, GenerationModality, GenerationStatus,
    };
    use sdkwork_intelligence_generations_service::ports::GenerationProvider;

    use crate::gateway::test_support::ScriptedGateway;
    use crate::image::ImageGenerationProviderAdapter;

    #[tokio::test]
    async fn openai_text_to_image_dispatch_maps_request_and_collects_usage() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.openai_image_generation.lock().unwrap() = Some(OpenAiImageList {
            created: 1,
            data: vec![
                OpenAiImage {
                    url: Some("https://cdn.example/a.png".to_string()),
                    b64_json: None,
                    mime_type: Some("image/png".to_string()),
                    revised_prompt: None,
                },
                OpenAiImage {
                    url: Some("https://cdn.example/b.png".to_string()),
                    b64_json: None,
                    mime_type: None,
                    revised_prompt: None,
                },
            ],
            usage: None,
        });
        let provider = ImageGenerationProviderAdapter::new(gateway.clone(), "openai");

        let record = sample_record("text_to_image");
        let command = CreateGenerationCommandRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "a cat astronaut".to_string(),
            model: Some("openai/gpt-image-2".to_string()),
            input_asset_ids: None,
            parameters: Some(serde_json::json!({
                "generationConfig": { "imageCount": 2, "aspectRatio": "1:1" }
            })),
        };
        let context = GenerationsRequestContext::from_parts("tenant-1".to_string(), "user-1".to_string(), "trace-1".to_string());
        let outcome = provider
            .dispatch(&record, &command, &context)
            .await
            .expect("dispatch succeeds");

        let request = gateway
            .last_openai_image_request
            .lock()
            .unwrap()
            .clone()
            .expect("openai request captured");
        assert_eq!(request.model, "gpt-image-2");
        assert_eq!(request.n, Some(2));
        assert_eq!(request.prompt, "a cat astronaut");

        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.record.result_count, 2);
        assert_eq!(outcome.results.len(), 2);
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.kind.as_deref(), Some("image"));
        assert_eq!(snapshot.url.as_deref(), Some("https://cdn.example/a.png"));
        let usage = outcome.usage.expect("usage extracted");
        assert_eq!(usage.vendor, "openai");
        assert_eq!(usage.image_count, 2);
    }

    #[tokio::test]
    async fn openai_image_edit_requires_reference_image() {
        let provider = ImageGenerationProviderAdapter::new(Arc::new(ScriptedGateway::default()), "openai");
        let record = sample_record("image_edit");
        let command = CreateGenerationCommandRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "restyle".to_string(),
            model: Some("openai/gpt-image-2".to_string()),
            input_asset_ids: None,
            parameters: None,
        };
        let error = provider
            .dispatch(&record, &command, &GenerationsRequestContext::from_parts("tenant-1".to_string(), "user-1".to_string(), "trace-1".to_string()))
            .await
            .expect_err("image edit without references must fail");
        assert!(error.to_string().contains("reference image"));
    }

    #[tokio::test]
    async fn kling_vendor_routes_to_kling_surface() {
        let provider = ImageGenerationProviderAdapter::new(Arc::new(ScriptedGateway::default()), "openai");
        let record = sample_record("text_to_image");
        let command = CreateGenerationCommandRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "neon city".to_string(),
            model: Some("kling/kling-image-v1".to_string()),
            input_asset_ids: None,
            parameters: None,
        };
        // The scripted gateway has no kling binding wired, so the dispatch
        // surfaces the gateway error rather than hitting the OpenAI surface.
        let error = provider
            .dispatch(&record, &command, &GenerationsRequestContext::from_parts("tenant-1".to_string(), "user-1".to_string(), "trace-1".to_string()))
            .await
            .expect_err("kling dispatch without script must fail");
        assert!(error.to_string().contains("599") || error.to_string().contains("scripted"));
    }

    pub(super) fn sample_record(operation_type: &str) -> sdkwork_intelligence_generations_service::domain::models::GenerationRecord {
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
}

#[cfg(test)]
mod sfx_tests {
    use std::sync::Arc;

    use cloudrouter_open_sdk::models::ElevenLabsSoundGenerationResponse;
    use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
    use sdkwork_intelligence_generations_service::domain::models::{
        CreateGenerationCommandRequest, GenerationModality, GenerationStatus,
    };
    use sdkwork_intelligence_generations_service::ports::GenerationProvider;

    use crate::gateway::test_support::ScriptedGateway;
    use crate::sfx::SoundEffectGenerationProviderAdapter;

    #[test]
    fn elevenlabs_sound_effect_dispatch_collects_audio_result_and_usage() {
        let gateway = Arc::new(ScriptedGateway::default());
        *gateway.elevenlabs_sound_generation.lock().unwrap() = Some(
            ElevenLabsSoundGenerationResponse {
                id: Some("task-1".to_string()),
                status: Some("completed".to_string()),
                audio_url: Some("https://cdn.example/sfx.mp3".to_string()),
                url: None,
                audio: None,
            },
        );
        let provider = SoundEffectGenerationProviderAdapter::new(gateway, "elevenlabs");
        let record = super::dispatch_tests::sample_record("sound_effects");
        let command = CreateGenerationCommandRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "door slam".to_string(),
            model: None,
            input_asset_ids: None,
            parameters: Some(serde_json::json!({
                "generationConfig": { "durationSeconds": 4, "loop": true }
            })),
        };
        let outcome = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(provider.dispatch(
                &record,
                &command,
                &GenerationsRequestContext::from_parts(
                    "tenant-1".to_string(),
                    "user-1".to_string(),
                    "trace-1".to_string(),
                ),
            ))
            .expect("dispatch succeeds");

        assert_eq!(outcome.record.status, GenerationStatus::Succeeded);
        assert_eq!(outcome.results.len(), 1);
        assert_eq!(outcome.results[0].result_type, "sound_effect");
        let snapshot = outcome.results[0].resource_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.url.as_deref(), Some("https://cdn.example/sfx.mp3"));
        let usage = outcome.usage.expect("usage extracted");
        assert_eq!(usage.vendor, "elevenlabs");
        assert_eq!(usage.audio_seconds, 4.0);
    }
}
