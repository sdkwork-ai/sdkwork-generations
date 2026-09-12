//! Typed gateway over the generated cloudrouter Rust SDK.
//!
//! [`MediaSdkGateway`] is the seam the vendor adapters depend on; the
//! production implementation wraps [`cloudrouter_open_sdk::SdkworkAiClient`].
//! Vendor surfaces that are not yet bound in the generated SDK (Kling image
//! generation, Volcengine image generation, Google Veo video generation) are
//! issued through the SDK's authenticated HTTP transport with request/response
//! models owned here.

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

    async fn gemini_create_video_generation(
        &self,
        model: &str,
        body: &GeminiVideoGenerationRequest,
    ) -> Result<GeminiVideoOperation, SdkworkError> {
        Err(SdkworkError::HttpStatus {
            status: 599,
            body: "gateway method not wired in this test double".to_string(),
        })
    }

    async fn gemini_retrieve_video_operation(
        &self,
        operation_name: &str,
    ) -> Result<GeminiVideoOperation, SdkworkError> {
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

    async fn gemini_create_video_generation(
        &self,
        model: &str,
        body: &GeminiVideoGenerationRequest,
    ) -> Result<GeminiVideoOperation, SdkworkError> {
        let path = format!(
            "/google/v1beta/models/{}:generateVideos",
            encode_path_segment(model)
        );
        self.client
            .http_client()
            .post(&ai_path(&path), Some(body), None, None, Some("application/json"))
            .await
    }

    async fn gemini_retrieve_video_operation(
        &self,
        operation_name: &str,
    ) -> Result<GeminiVideoOperation, SdkworkError> {
        // The long-running operation name is a relative resource path
        // (`models/{model}/operations/{id}`); its separators must survive.
        let path = format!("/google/v1beta/{}", operation_name.trim().trim_start_matches('/'));
        self.client.http_client().get(&ai_path(&path), None, None).await
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

/// One Veo generation instance in the Gemini `generateVideos` envelope.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiVideoInstance {
    pub prompt: String,
    /// Optional reference image. The Gemini API expects base64-encoded bytes
    /// (`image.bytesBase64Encoded`); the generations command plane carries
    /// image URLs only, so image-to-video instances are rejected upstream of
    /// this envelope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<serde_json::Value>,
}

/// Tuning parameters for a Veo generation request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiVideoParameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person_generation: Option<String>,
}

/// Google Veo video generation request (`models/{model}:generateVideos`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiVideoGenerationRequest {
    pub instances: Vec<GeminiVideoInstance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<GeminiVideoParameters>,
}

/// Generated video sample returned by a finished Veo operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiVideoSample {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<GeminiVideoAsset>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "gcsUri")]
    pub gcs_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiVideoAsset {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiVideoOperationResponse {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "generateVideoResponse")]
    pub generate_video_response: Option<GeminiVideoGenerateResponse>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiVideoGenerateResponse {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "generatedSamples")]
    pub generated_samples: Option<Vec<GeminiVideoSample>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub videos: Option<Vec<GeminiVideoSample>>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "raiMediaFilteredReasons")]
    pub rai_media_filtered_reasons: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiVideoOperationError {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Google Veo long-running operation envelope (`name`/`done`/`response`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiVideoOperation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<GeminiVideoOperationResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<GeminiVideoOperationError>,
}

impl GeminiVideoOperation {
    /// Operation resource name, usable as the polling identifier.
    pub fn operation_name(&self) -> Option<&str> {
        self.name.as_deref().filter(|value| !value.trim().is_empty())
    }

    /// Video URIs from a finished operation, if any.
    pub fn video_uris(&self) -> Vec<String> {
        let Some(response) = self.response.as_ref() else {
            return Vec::new();
        };
        let Some(generate_video_response) = response.generate_video_response.as_ref() else {
            return Vec::new();
        };
        let samples = generate_video_response
            .generated_samples
            .as_ref()
            .or(generate_video_response.videos.as_ref());
        samples
            .into_iter()
            .flatten()
            .filter_map(|sample| {
                sample
                    .video
                    .as_ref()
                    .and_then(|video| video.uri.clone().or_else(|| video.url.clone()))
                    .or_else(|| sample.uri.clone())
                    .or_else(|| sample.url.clone())
                    .or_else(|| sample.gcs_uri.clone())
            })
            .filter(|uri| !uri.trim().is_empty())
            .collect()
    }

    /// Safety-filter failure message when the operation finished without media.
    pub fn filtered_reason(&self) -> Option<String> {
        self.response
            .as_ref()
            .and_then(|response| response.generate_video_response.as_ref())
            .and_then(|generate_video_response| generate_video_response.rai_media_filtered_reasons.as_ref())
            .and_then(|reasons| reasons.first().cloned())
    }
}

/// Percent-encodes a single path segment, preserving the unreserved set so
/// gateway model ids travel without ambiguation.
fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.trim().as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(*byte as char)
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
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
        pub gemini_video_operation: Mutex<Option<GeminiVideoOperation>>,
        pub last_gemini_video_request: Mutex<Option<(String, GeminiVideoGenerationRequest)>>,
        pub last_gemini_video_operation_name: Mutex<Option<String>>,
        pub nano_banana_create_task: Mutex<Option<NanoBananaImageGenerationTask>>,
        pub last_nano_banana_create_request: Mutex<Option<NanoBananaImageGenerationRequest>>,
        pub nano_banana_retrieve_task: Mutex<Option<NanoBananaImageGenerationTask>>,
        pub kling_video_create_task: Mutex<Option<KlingVideoGenerationTask>>,
        pub last_kling_video_create_request: Mutex<Option<KlingVideoGenerationRequest>>,
        pub kling_video_retrieve_task: Mutex<Option<KlingVideoGenerationTask>>,
        pub openai_video_create: Mutex<Option<OpenAiVideo>>,
        pub openai_video_retrieve: Mutex<Option<OpenAiVideo>>,
    }

    impl Default for ScriptedGateway {
        fn default() -> Self {
            Self {
                openai_image_generation: Mutex::new(None),
                last_openai_image_request: Mutex::new(None),
                elevenlabs_sound_generation: Mutex::new(None),
                gemini_video_operation: Mutex::new(None),
                last_gemini_video_request: Mutex::new(None),
                last_gemini_video_operation_name: Mutex::new(None),
                nano_banana_create_task: Mutex::new(None),
                last_nano_banana_create_request: Mutex::new(None),
                nano_banana_retrieve_task: Mutex::new(None),
                kling_video_create_task: Mutex::new(None),
                last_kling_video_create_request: Mutex::new(None),
                kling_video_retrieve_task: Mutex::new(None),
                openai_video_create: Mutex::new(None),
                openai_video_retrieve: Mutex::new(None),
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

        async fn gemini_create_video_generation(
            &self,
            model: &str,
            body: &GeminiVideoGenerationRequest,
        ) -> Result<GeminiVideoOperation, SdkworkError> {
            *self.last_gemini_video_request.lock().unwrap() = Some((model.to_string(), body.clone()));
            self.gemini_video_operation
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted gemini video operation response".to_string(),
                })
        }

        async fn gemini_retrieve_video_operation(
            &self,
            operation_name: &str,
        ) -> Result<GeminiVideoOperation, SdkworkError> {
            *self.last_gemini_video_operation_name.lock().unwrap() = Some(operation_name.to_string());
            self.gemini_video_operation
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted gemini video operation response".to_string(),
                })
        }

        async fn nano_banana_create_image_generation(
            &self,
            body: &NanoBananaImageGenerationRequest,
        ) -> Result<NanoBananaImageGenerationTask, SdkworkError> {
            *self.last_nano_banana_create_request.lock().unwrap() = Some(body.clone());
            self.nano_banana_create_task
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted nano-banana create response".to_string(),
                })
        }

        async fn nano_banana_retrieve_image_generation(
            &self,
            _task_id: &str,
        ) -> Result<NanoBananaImageGenerationTask, SdkworkError> {
            self.nano_banana_retrieve_task
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted nano-banana retrieve response".to_string(),
                })
        }

        async fn kling_create_video_generation(
            &self,
            body: &KlingVideoGenerationRequest,
        ) -> Result<KlingVideoGenerationTask, SdkworkError> {
            *self.last_kling_video_create_request.lock().unwrap() = Some(body.clone());
            self.kling_video_create_task
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted kling video create response".to_string(),
                })
        }

        async fn kling_retrieve_video_generation(
            &self,
            _task_id: &str,
        ) -> Result<KlingVideoGenerationTask, SdkworkError> {
            self.kling_video_retrieve_task
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted kling video retrieve response".to_string(),
                })
        }

        async fn openai_create_video(
            &self,
            _body: &OpenAiVideoCreateRequest,
        ) -> Result<OpenAiVideo, SdkworkError> {
            self.openai_video_create
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted openai video create response".to_string(),
                })
        }

        async fn openai_retrieve_video(&self, _video_id: &str) -> Result<OpenAiVideo, SdkworkError> {
            self.openai_video_retrieve
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| SdkworkError::HttpStatus {
                    status: 599,
                    body: "no scripted openai video retrieve response".to_string(),
                })
        }
    }
}
