//! Typed gateway over the generated cloudrouter Rust SDK.
//!
//! [`MediaSdkGateway`] is the seam the vendor adapters depend on; the
//! production implementation wraps [`cloudrouter_open_sdk::SdkworkAiClient`].
//! Vendor surfaces that are not yet bound in the generated SDK (Kling image
//! generation, Volcengine image generation) are issued through the SDK's
//! authenticated HTTP transport with request/response models owned here.

use async_trait::async_trait;
use cloudrouter_open_sdk::api::paths::ai_path;
use cloudrouter_open_sdk::models::{
    ElevenLabsSoundGenerationRequest, ElevenLabsSoundGenerationResponse,
    ElevenLabsTextToSpeechRequest, ElevenLabsTextToSpeechResponse, KlingVideoGenerationRequest,
    KlingVideoGenerationTask, NanoBananaImageGenerationRequest,
    NanoBananaImageGenerationTask, OpenAiAudioTranscription, OpenAiAudioTranscriptionRequest,
    OpenAiAudioTranslation, OpenAiAudioTranslationRequest, OpenAiImageEditRequest,
    OpenAiImageGenerationRequest, OpenAiImageList, OpenAiSpeechCreateRequest, OpenAiVideo,
    OpenAiVideoCreateRequest, OpenAiVideoExtendRequest, ProviderGeneratedMedia, ProviderTaskError,
    SunoMusicGenerationRequest, SunoMusicGenerationResponse, SunoMusicGenerationTaskResponse,
    ViduImageGenerationTask, ViduImageToVideoRequest, ViduReferenceToImageRequest,
    ViduStartEndToVideoRequest, ViduTaskCreationsResponse, ViduTextToVideoRequest,
    ViduVideoGenerationTask, VolcengineContentGenerationTask,
    VolcengineContentGenerationTaskCreateRequest, VolcengineContentGenerationTaskCreateResponse,
};
use cloudrouter_open_sdk::{SdkworkAiClient, SdkworkError};
use serde::{Deserialize, Serialize};

/// Connection settings for the cloudrouter media gateway.
#[derive(Debug, Clone)]
pub struct GatewaySettings {
    /// Cloudrouter open-api base URL (OpenAI-compatible gateway).
    pub base_url: String,
    /// API key used for account-pool routing.
    pub api_key: Option<String>,
    /// End-user auth token used for account routing (takes precedence).
    pub auth_token: Option<String>,
    /// Optional access token for dual-token routing.
    pub access_token: Option<String>,
}

impl GatewaySettings {
    /// Load gateway settings from the environment.
    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("GENERATIONS_MEDIA_GATEWAY_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3900".to_string()),
            api_key: std::env::var("GENERATIONS_MEDIA_GATEWAY_API_KEY").ok(),
            auth_token: std::env::var("GENERATIONS_MEDIA_GATEWAY_AUTH_TOKEN").ok(),
            access_token: std::env::var("GENERATIONS_MEDIA_GATEWAY_ACCESS_TOKEN").ok(),
        }
    }
}

/// Vendor media SDK seam used by the generation providers.
#[async_trait]
#[allow(unused_variables)]
pub trait MediaSdkGateway: Send + Sync {
    // -- Image --------------------------------------------------------------

    async fn openai_create_image_generation(
        &self,
        body: &OpenAiImageGenerationRequest,
    ) -> Result<OpenAiImageList, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn openai_create_image_edit(
        &self,
        body: &OpenAiImageEditRequest,
    ) -> Result<OpenAiImageList, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn nano_banana_create_image_generation(
        &self,
        body: &NanoBananaImageGenerationRequest,
    ) -> Result<NanoBananaImageGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn nano_banana_retrieve_image_generation(
        &self,
        task_id: &str,
    ) -> Result<NanoBananaImageGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn vidu_create_reference_to_image(
        &self,
        body: &ViduReferenceToImageRequest,
    ) -> Result<ViduImageGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn vidu_retrieve_image_creations(
        &self,
        task_id: &str,
    ) -> Result<ViduTaskCreationsResponse, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn kling_create_image_generation(
        &self,
        body: &KlingImageGenerationRequest,
    ) -> Result<ProviderImageGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn kling_retrieve_image_generation(
        &self,
        task_id: &str,
    ) -> Result<ProviderImageGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn volcengine_create_image_generation(
        &self,
        body: &VolcengineImageGenerationRequest,
    ) -> Result<VolcengineImageGenerationResponse, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn elevenlabs_create_sound_generation(
        &self,
        body: &ElevenLabsSoundGenerationRequest,
        output_format: Option<&str>,
    ) -> Result<ElevenLabsSoundGenerationResponse, SdkworkError> {
        let _ = (body, output_format);
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn elevenlabs_create_text_to_speech(
        &self,
        voice_id: &str,
        body: &ElevenLabsTextToSpeechRequest,
        output_format: Option<&str>,
    ) -> Result<ElevenLabsTextToSpeechResponse, SdkworkError> {
        let _ = (voice_id, body, output_format);
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    // -- Video --------------------------------------------------------------

    async fn openai_create_video(
        &self,
        body: &OpenAiVideoCreateRequest,
    ) -> Result<OpenAiVideo, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn openai_retrieve_video(&self, video_id: &str) -> Result<OpenAiVideo, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn openai_create_video_extension(
        &self,
        body: &OpenAiVideoExtendRequest,
    ) -> Result<OpenAiVideo, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn kling_create_video_generation(
        &self,
        body: &KlingVideoGenerationRequest,
    ) -> Result<KlingVideoGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn kling_retrieve_video_generation(
        &self,
        task_id: &str,
    ) -> Result<KlingVideoGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn vidu_create_text_to_video(
        &self,
        body: &ViduTextToVideoRequest,
    ) -> Result<ViduVideoGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn vidu_create_image_to_video(
        &self,
        body: &ViduImageToVideoRequest,
    ) -> Result<ViduVideoGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn vidu_create_start_end_to_video(
        &self,
        body: &ViduStartEndToVideoRequest,
    ) -> Result<ViduVideoGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn vidu_retrieve_video_creations(
        &self,
        task_id: &str,
    ) -> Result<ViduTaskCreationsResponse, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn volcengine_create_video_task(
        &self,
        body: &VolcengineContentGenerationTaskCreateRequest,
    ) -> Result<VolcengineContentGenerationTaskCreateResponse, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn volcengine_retrieve_video_task(
        &self,
        task_id: &str,
    ) -> Result<VolcengineContentGenerationTask, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    // -- Music --------------------------------------------------------------

    async fn suno_create_music_generation(
        &self,
        body: &SunoMusicGenerationRequest,
    ) -> Result<SunoMusicGenerationResponse, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn suno_retrieve_music_generation(
        &self,
        task_id: &str,
    ) -> Result<SunoMusicGenerationTaskResponse, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    // -- Voice --------------------------------------------------------------

    async fn openai_create_speech(
        &self,
        body: &OpenAiSpeechCreateRequest,
    ) -> Result<Vec<u8>, SdkworkError> {
        let _ = body;
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn openai_create_transcription(
        &self,
        body: &OpenAiAudioTranscriptionRequest,
    ) -> Result<OpenAiAudioTranscription, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn openai_create_translation(
        &self,
        body: &OpenAiAudioTranslationRequest,
    ) -> Result<OpenAiAudioTranslation, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }
}

/// Production gateway backed by the generated cloudrouter Rust SDK.
pub struct CloudRouterMediaGateway {
    client: SdkworkAiClient,
}

impl CloudRouterMediaGateway {
    /// Build a gateway from explicit settings.
    pub fn new(settings: &GatewaySettings) -> Result<Self, SdkworkError> {
        let client = SdkworkAiClient::new_with_base_url(settings.base_url.clone())?;
        if let Some(auth_token) = settings.auth_token.as_deref().filter(|v| !v.trim().is_empty()) {
            client.set_auth_token(auth_token);
        }
        if let Some(access_token) = settings
            .access_token
            .as_deref()
            .filter(|v| !v.trim().is_empty())
        {
            client.set_access_token(access_token);
        }
        if let Some(api_key) = settings.api_key.as_deref().filter(|v| !v.trim().is_empty()) {
            client.set_api_key(api_key);
        }
        Ok(Self { client })
    }

    /// Build a gateway from environment settings.
    pub fn from_env() -> Result<Self, SdkworkError> {
        Self::new(&GatewaySettings::from_env())
    }
}

#[async_trait]
impl MediaSdkGateway for CloudRouterMediaGateway {
    async fn openai_create_image_generation(
        &self,
        body: &OpenAiImageGenerationRequest,
    ) -> Result<OpenAiImageList, SdkworkError> {
        self.client.images().create_generation(body).await
    }

    async fn openai_create_image_edit(
        &self,
        body: &OpenAiImageEditRequest,
    ) -> Result<OpenAiImageList, SdkworkError> {
        self.client.images().create_edit(body).await
    }

    async fn nano_banana_create_image_generation(
        &self,
        body: &NanoBananaImageGenerationRequest,
    ) -> Result<NanoBananaImageGenerationTask, SdkworkError> {
        self.client.images_nano_banana().create_generations(body).await
    }

    async fn nano_banana_retrieve_image_generation(
        &self,
        task_id: &str,
    ) -> Result<NanoBananaImageGenerationTask, SdkworkError> {
        self.client
            .images_nano_banana()
            .retrieve_generations(task_id)
            .await
    }

    async fn vidu_create_reference_to_image(
        &self,
        body: &ViduReferenceToImageRequest,
    ) -> Result<ViduImageGenerationTask, SdkworkError> {
        self.client
            .images_vidu()
            .create_ent_v2_reference2image(body)
            .await
    }

    async fn vidu_retrieve_image_creations(
        &self,
        task_id: &str,
    ) -> Result<ViduTaskCreationsResponse, SdkworkError> {
        self.client.videos_vidu().list_ent_v2_tasks_creations(task_id).await
    }

    async fn kling_create_image_generation(
        &self,
        body: &KlingImageGenerationRequest,
    ) -> Result<ProviderImageGenerationTask, SdkworkError> {
        self.client
            .http_client()
            .post(&ai_path("/kling/v1/images/generations"), Some(body), None, None, Some("application/json"))
            .await
    }

    async fn kling_retrieve_image_generation(
        &self,
        task_id: &str,
    ) -> Result<ProviderImageGenerationTask, SdkworkError> {
        self.client
            .http_client()
            .get(
                &ai_path(&format!("/kling/v1/tasks/{task_id}")),
                None,
                None,
            )
            .await
    }

    async fn volcengine_create_image_generation(
        &self,
        body: &VolcengineImageGenerationRequest,
    ) -> Result<VolcengineImageGenerationResponse, SdkworkError> {
        self.client
            .http_client()
            .post(
                &ai_path("/volcengine/api/v3/images/generations"),
                Some(body),
                None,
                None,
                Some("application/json"),
            )
            .await
    }

    async fn elevenlabs_create_sound_generation(
        &self,
        body: &ElevenLabsSoundGenerationRequest,
        output_format: Option<&str>,
    ) -> Result<ElevenLabsSoundGenerationResponse, SdkworkError> {
        self.client
            .audio_elevenlabs()
            .create_v1_sound_generation(body, output_format)
            .await
    }

    async fn elevenlabs_create_text_to_speech(
        &self,
        voice_id: &str,
        body: &ElevenLabsTextToSpeechRequest,
        output_format: Option<&str>,
    ) -> Result<ElevenLabsTextToSpeechResponse, SdkworkError> {
        self.client
            .audio_elevenlabs()
            .create_v1_text_to_speech(voice_id, body, output_format)
            .await
    }

    async fn openai_create_video(
        &self,
        body: &OpenAiVideoCreateRequest,
    ) -> Result<OpenAiVideo, SdkworkError> {
        self.client.video().create(body).await
    }

    async fn openai_retrieve_video(&self, video_id: &str) -> Result<OpenAiVideo, SdkworkError> {
        self.client.video().retrieve(video_id).await
    }

    async fn openai_create_video_extension(
        &self,
        body: &OpenAiVideoExtendRequest,
    ) -> Result<OpenAiVideo, SdkworkError> {
        self.client.video().create_extension(body).await
    }

    async fn kling_create_video_generation(
        &self,
        body: &KlingVideoGenerationRequest,
    ) -> Result<KlingVideoGenerationTask, SdkworkError> {
        self.client.videos_kling().create_v1_videos_generation(body).await
    }

    async fn kling_retrieve_video_generation(
        &self,
        task_id: &str,
    ) -> Result<KlingVideoGenerationTask, SdkworkError> {
        self.client
            .videos_kling()
            .list_v1_videos_generations(task_id)
            .await
    }

    async fn vidu_create_text_to_video(
        &self,
        body: &ViduTextToVideoRequest,
    ) -> Result<ViduVideoGenerationTask, SdkworkError> {
        self.client.videos_vidu().create_ent_v2_text2video(body).await
    }

    async fn vidu_create_image_to_video(
        &self,
        body: &ViduImageToVideoRequest,
    ) -> Result<ViduVideoGenerationTask, SdkworkError> {
        self.client.videos_vidu().create_ent_v2_img2video(body).await
    }

    async fn vidu_create_start_end_to_video(
        &self,
        body: &ViduStartEndToVideoRequest,
    ) -> Result<ViduVideoGenerationTask, SdkworkError> {
        self.client
            .videos_vidu()
            .create_ent_v2_start_end2video(body)
            .await
    }

    async fn vidu_retrieve_video_creations(
        &self,
        task_id: &str,
    ) -> Result<ViduTaskCreationsResponse, SdkworkError> {
        self.client.videos_vidu().list_ent_v2_tasks_creations(task_id).await
    }

    async fn volcengine_create_video_task(
        &self,
        body: &VolcengineContentGenerationTaskCreateRequest,
    ) -> Result<VolcengineContentGenerationTaskCreateResponse, SdkworkError> {
        self.client
            .videos_volcengine()
            .create_api_v3_contents_generations_task(body)
            .await
    }

    async fn volcengine_retrieve_video_task(
        &self,
        task_id: &str,
    ) -> Result<VolcengineContentGenerationTask, SdkworkError> {
        self.client
            .videos_volcengine()
            .list_api_v3_contents_generations_tasks(task_id)
            .await
    }

    async fn suno_create_music_generation(
        &self,
        body: &SunoMusicGenerationRequest,
    ) -> Result<SunoMusicGenerationResponse, SdkworkError> {
        self.client.audio_suno().create_v1_music_generation(body).await
    }

    async fn suno_retrieve_music_generation(
        &self,
        task_id: &str,
    ) -> Result<SunoMusicGenerationTaskResponse, SdkworkError> {
        self.client.audio_suno().list_v1_music_generations(task_id).await
    }

    async fn openai_create_speech(
        &self,
        body: &OpenAiSpeechCreateRequest,
    ) -> Result<Vec<u8>, SdkworkError> {
        self.client.audio().create_speech(body).await
    }

    async fn openai_create_transcription(
        &self,
        body: &OpenAiAudioTranscriptionRequest,
    ) -> Result<OpenAiAudioTranscription, SdkworkError> {
        self.client.audio().create_transcription(body).await
    }

    async fn openai_create_translation(
        &self,
        body: &OpenAiAudioTranslationRequest,
    ) -> Result<OpenAiAudioTranslation, SdkworkError> {
        self.client.audio().create_translation(body).await
    }
}

/// Kling image generation request (native Kling open API shape).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KlingImageGenerationRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_reference_list: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

/// Canonical provider image task envelope returned by the gateway for
/// task-based image vendors (Kling). Mirrors the nano-banana task schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderImageGenerationTask {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ProviderGeneratedMedia>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ProviderTaskError>,
}

/// Volcengine Ark image generation request (OpenAI-compatible shape).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolcengineImageGenerationRequest {
    pub model: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watermark: Option<bool>,
}

/// Volcengine Ark image generation response (OpenAI-compatible shape).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolcengineImageGenerationResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<VolcengineImageData>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<VolcengineImageUsage>,
}

/// One generated image in a Volcengine Ark response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolcengineImageData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
}

/// Volcengine Ark image usage block.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolcengineImageUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_images: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i64>,
}

/// Test seam capturing the last request and returning a scripted response.
pub mod test_support {
    use async_trait::async_trait;
    use std::sync::Mutex;

    use super::*;

    /// Scripted gateway: overrides only the vendor calls a test exercises.
    pub struct ScriptedGateway {
        pub openai_image_generation: Mutex<Option<OpenAiImageList>>,
        pub last_openai_image_request: Mutex<Option<OpenAiImageGenerationRequest>>,
        pub elevenlabs_sound_generation: Mutex<Option<ElevenLabsSoundGenerationResponse>>,
    }

    impl Default for ScriptedGateway {
        fn default() -> Self {
            Self {
                openai_image_generation: Mutex::new(None),
                last_openai_image_request: Mutex::new(None),
                elevenlabs_sound_generation: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl MediaSdkGateway for ScriptedGateway {
        async fn openai_create_image_generation(
            &self,
            body: &OpenAiImageGenerationRequest,
        ) -> Result<OpenAiImageList, SdkworkError> {
            *self.last_openai_image_request.lock().unwrap() = Some(body.clone());
            self.openai_image_generation
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted openai image generation response".to_string(),
                })
        }

        async fn elevenlabs_create_sound_generation(
            &self,
            _body: &ElevenLabsSoundGenerationRequest,
            _output_format: Option<&str>,
        ) -> Result<ElevenLabsSoundGenerationResponse, SdkworkError> {
            self.elevenlabs_sound_generation
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted elevenlabs sound generation response".to_string(),
                })
        }
    }
}
