//! Vendor identity resolution and command input extraction.
//!
//! Vendors are resolved from the model catalog key (`vendor/model`), the
//! `vendor` parameter, or the modality default. All vendor parameter bags are
//! extracted from the command `parameters` object with vendor-specific names
//! kept alongside the shared generation config keys.

use serde_json::Value;
use sdkwork_intelligence_generations_service::domain::models::CreateGenerationCommandRequest;

/// Canonical vendor slugs supported by the adapters.
pub const VENDOR_OPENAI: &str = "openai";
pub const VENDOR_NANO_BANANA: &str = "nano-banana";
pub const VENDOR_GOOGLE: &str = "google";
pub const VENDOR_KLING: &str = "kling";
pub const VENDOR_VOLCENGINE: &str = "volcengine";
pub const VENDOR_JIMENG: &str = "jimeng";
pub const VENDOR_VIDU: &str = "vidu";
pub const VENDOR_MIDJOURNEY: &str = "midjourney";
pub const VENDOR_SUNO: &str = "suno";
pub const VENDOR_ELEVENLABS: &str = "elevenlabs";

/// Resolved vendor selection for a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSelection {
    /// Vendor slug (for example `openai`, `kling`).
    pub vendor: String,
    /// Provider model id with the vendor prefix stripped.
    pub model: String,
}

/// Resolve the vendor for a command.
///
/// Precedence: explicit `parameters.vendor` > `vendor/` model prefix > default
/// vendor for the modality.
pub fn resolve_vendor(command: &CreateGenerationCommandRequest, default_vendor: &str) -> VendorSelection {
    let raw_model = command.model.as_deref().unwrap_or_default().trim();
    let parameters = command.parameters.as_ref();
    let explicit_vendor = parameters
        .and_then(|params| params.get("vendor"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(vendor) = explicit_vendor {
        return VendorSelection {
            vendor: normalize_vendor(vendor),
            model: raw_model.to_string(),
        };
    }

    if let Some((prefix, model)) = raw_model.split_once('/') {
        if !prefix.trim().is_empty() && !model.trim().is_empty() {
            return VendorSelection {
                vendor: normalize_vendor(prefix),
                model: model.trim().to_string(),
            };
        }
    }

    VendorSelection {
        vendor: normalize_vendor(default_vendor),
        model: raw_model.to_string(),
    }
}

/// Normalize a vendor alias into the canonical slug.
///
/// The Google family (Gemini image via nano-banana, Veo video) collapses onto
/// `nano-banana`; the modality adapters map that slug onto their own Google
/// surface.
pub fn normalize_vendor(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "google" | "gemini" | "veo" => VENDOR_NANO_BANANA.to_string(),
        "bytedance" | "byte_dance" | "volces" | "ark" | "doubao" | "seedream" | "seedance" => {
            VENDOR_VOLCENGINE.to_string()
        }
        "jimeng" => VENDOR_JIMENG.to_string(),
        "eleven-labs" | "11labs" => VENDOR_ELEVENLABS.to_string(),
        _ => normalized,
    }
}

/// Extracted command inputs shared across modality adapters.
#[derive(Debug, Clone, Default)]
pub struct GenerationCommandInputs {
    pub prompt: String,
    pub model: String,
    pub image_count: Option<i64>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub response_format: Option<String>,
    pub aspect_ratio: Option<String>,
    pub resolution: Option<String>,
    pub duration_seconds: Option<f64>,
    pub seed: Option<i64>,
    pub negative_prompt: Option<String>,
    pub voice: Option<String>,
    pub voice_id: Option<String>,
    pub speed: Option<f64>,
    pub stability: Option<f64>,
    pub similarity: Option<f64>,
    pub style: Option<f64>,
    pub language: Option<String>,
    pub emotion: Option<String>,
    pub is_instrumental: Option<bool>,
    pub lyrics_optimizer: Option<bool>,
    pub sample_rate: Option<i64>,
    pub audio_bitrate: Option<i64>,
    pub audio_format: Option<String>,
    pub tags: Option<String>,
    pub title: Option<String>,
    pub lyrics: Option<String>,
    pub negative_tags: Option<String>,
    pub mode: Option<String>,
    pub cfg_scale: Option<f64>,
    pub loop_enabled: Option<bool>,
    pub reference_images: Vec<String>,
    pub reference_image_tail: Option<String>,
    pub reference_videos: Vec<String>,
    pub audio_url: Option<String>,
    pub audio_text: Option<String>,
    pub input_asset_ids: Vec<String>,
}

impl GenerationCommandInputs {
    /// Extract command inputs from a creation request.
    ///
    /// `selection` is required rather than optional: the wire model is the
    /// *vendor-stripped* model id, which only [`resolve_vendor`] knows (it
    /// removes a leading `vendor/` prefix resolved from `parameters.vendor` or
    /// the model prefix). Deriving it here would leak the prefix upstream, and
    /// defaulting it to empty would make every adapter's `model_or_default`
    /// fallback silently replace the caller's model with a hardcoded default —
    /// and make `(!inputs.model.is_empty()).then(...)` drop the field from the
    /// request body entirely. Taking the selection as a parameter makes that
    /// mistake impossible to reintroduce at a new call site.
    pub fn from_command(
        command: &CreateGenerationCommandRequest,
        selection: &VendorSelection,
    ) -> Self {
        let parameters = command.parameters.clone().unwrap_or(Value::Null);
        let parameters = parameters.as_object();
        let generation_config = parameters
            .and_then(|params| {
                params
                    .get("generationConfig")
                    .or_else(|| params.get("generation_config"))
            })
            .cloned()
            .unwrap_or(Value::Null);
        let generation_config = generation_config.as_object();

        let string_from = |keys: &[&str]| -> Option<String> {
            for source in [parameters, generation_config] {
                let Some(source) = source else { continue };
                for key in keys {
                    if let Some(value) = source.get(*key) {
                        if let Some(text) = value.as_str() {
                            let text = text.trim();
                            if !text.is_empty() {
                                return Some(text.to_string());
                            }
                        }
                    }
                }
            }
            None
        };
        let number_from = |keys: &[&str]| -> Option<f64> {
            for source in [parameters, generation_config] {
                let Some(source) = source else { continue };
                for key in keys {
                    if let Some(value) = source.get(*key) {
                        if let Some(number) = value.as_f64() {
                            return Some(number);
                        }
                    }
                }
            }
            None
        };
        let bool_from = |keys: &[&str]| -> Option<bool> {
            for source in [parameters, generation_config] {
                let Some(source) = source else { continue };
                for key in keys {
                    if let Some(value) = source.get(*key) {
                        if let Some(flag) = value.as_bool() {
                            return Some(flag);
                        }
                    }
                }
            }
            None
        };

        let reference_images = parameters
            .and_then(|params| {
                params
                    .get("referenceImages")
                    .or_else(|| params.get("reference_images"))
            })
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| {
                        entry
                            .get("url")
                            .or_else(|| entry.get("publicUrl"))
                            .or_else(|| entry.get("assetId"))
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let reference_assets = parameters
            .and_then(|params| {
                params
                    .get("referenceAssets")
                    .or_else(|| params.get("reference_assets"))
            })
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| {
                        entry
                            .get("url")
                            .or_else(|| entry.get("publicUrl"))
                            .or_else(|| entry.get("assetId"))
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let reference_videos = parameters
            .and_then(|params| {
                params
                    .get("referenceVideos")
                    .or_else(|| params.get("reference_videos"))
                    .or_else(|| params.get("videoUrls"))
                    .or_else(|| params.get("video_urls"))
                    .or_else(|| params.get("drivingVideos"))
                    .or_else(|| params.get("driving_videos"))
            })
            .and_then(|value| {
                value
                    .as_str()
                    .map(|url| vec![url.to_string()])
                    .or_else(|| {
                        value.as_array().map(|entries| {
                            entries
                                .iter()
                                .filter_map(|entry| {
                                    entry
                                        .get("url")
                                        .or_else(|| entry.get("videoUrl"))
                                        .and_then(serde_json::Value::as_str)
                                        .map(str::to_string)
                                })
                                .collect::<Vec<_>>()
                        })
                    })
            })
            .unwrap_or_default();

        Self {
            prompt: command.prompt.clone(),
            model: selection.model.clone(),
            image_count: number_from(&["imageCount", "image_count", "n"]).map(|value| value as i64),
            size: string_from(&["size", "imageSize"]),
            quality: string_from(&["quality"]),
            response_format: string_from(&["responseFormat", "response_format"]),
            aspect_ratio: string_from(&["aspectRatio", "aspect_ratio"]),
            resolution: string_from(&["resolution"]),
            duration_seconds: number_from(&["durationSeconds", "duration_seconds", "duration"]),
            seed: number_from(&["seed"]).map(|value| value as i64),
            negative_prompt: string_from(&["negativePrompt", "negative_prompt"]),
            voice: string_from(&["voice"]),
            voice_id: string_from(&["voiceId", "voice_id"]),
            speed: number_from(&["speed"]),
            stability: number_from(&["stability"]),
            similarity: number_from(&["similarity", "similarityBoost", "similarity_boost"]),
            style: number_from(&["style", "styleExaggeration"]),
            language: string_from(&["language", "languageCode", "language_code"]),
            emotion: string_from(&["emotion"]),
            is_instrumental: bool_from(&["isInstrumental", "is_instrumental", "instrumental"]),
            lyrics_optimizer: bool_from(&["lyricsOptimizer", "lyrics_optimizer"]),
            sample_rate: number_from(&["sampleRate", "sample_rate"]).map(|value| value as i64),
            audio_bitrate: number_from(&["bitrate", "audioBitrate", "audio_bitrate"])
                .map(|value| value as i64),
            audio_format: string_from(&["audioFormat", "audio_format", "format"]),
            tags: string_from(&["tags", "styleTags", "style_tags"]),
            title: string_from(&["title"]),
            lyrics: string_from(&["lyrics"]),
            negative_tags: string_from(&["negativeTags", "negative_tags"]),
            mode: string_from(&["mode"]),
            cfg_scale: number_from(&["cfgScale", "cfg_scale", "promptInfluence"]),
            loop_enabled: parameters
                .and_then(|params| {
                    params
                        .get("generationConfig")
                        .or_else(|| params.get("generation_config"))
                })
                .and_then(|config| config.get("loop"))
                .and_then(serde_json::Value::as_bool),
            reference_images,
            reference_image_tail: string_from(&["imageTail", "image_tail", "lastFrame"]),
            reference_videos,
            audio_url: string_from(&["audioUrl", "audio_url", "audio"]),
            audio_text: string_from(&["audioText", "audio_text"]),
            input_asset_ids: command.input_asset_ids.clone().unwrap_or_default(),
        }
        .with_reference_assets(reference_assets)
    }

    fn with_reference_assets(mut self, assets: Vec<String>) -> Self {
        self.reference_images.extend(assets);
        self
    }

    /// First reference image, if any.
    pub fn first_reference_image(&self) -> Option<String> {
        self.reference_images
            .iter()
            .find(|value| !value.trim().is_empty())
            .cloned()
    }

    /// First non-blank source video reference (`parameters.referenceVideos`,
    /// `videoUrls`, or `drivingVideos`).
    ///
    /// Video-extension and motion-transfer surfaces must send the *video* the
    /// caller supplied, never a reference image standing in for it.
    pub fn first_reference_video(&self) -> Option<String> {
        self.reference_videos
            .iter()
            .find(|value| !value.trim().is_empty())
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_vendor_from_model_prefix() {
        let command = command_with_model("kling/kling-image-v1");
        let selection = resolve_vendor(&command, "openai");
        assert_eq!(selection.vendor, "kling");
        assert_eq!(selection.model, "kling-image-v1");
    }

    #[test]
    fn resolves_vendor_from_parameters_and_normalizes_aliases() {
        let mut command = command_with_model("gpt-image-2");
        command.parameters = Some(serde_json::json!({ "vendor": "Google" }));
        let selection = resolve_vendor(&command, "openai");
        assert_eq!(selection.vendor, VENDOR_NANO_BANANA);
        assert_eq!(selection.model, "gpt-image-2");
    }

    #[test]
    fn falls_back_to_modality_default_vendor() {
        let command = command_with_model("gpt-image-2");
        let selection = resolve_vendor(&command, VENDOR_OPENAI);
        assert_eq!(selection.vendor, VENDOR_OPENAI);
        assert_eq!(selection.model, "gpt-image-2");
    }

    #[test]
    fn extracts_generation_config_and_references() {
        let mut command = command_with_model("openai/gpt-image-2");
        command.parameters = Some(serde_json::json!({
            "generationConfig": {
                "aspectRatio": "1:1",
                "imageCount": 2,
                "quality": "high"
            },
            "referenceImages": [
                { "url": "https://cdn.example/ref.png" }
            ],
            "vendor": "openai"
        }));
        command.input_asset_ids = Some(vec!["drive://space/asset-1".to_string()]);

        let selection = resolve_vendor(&command, VENDOR_OPENAI);
        let inputs = GenerationCommandInputs::from_command(&command, &selection);
        assert_eq!(inputs.image_count, Some(2));
        assert_eq!(inputs.aspect_ratio.as_deref(), Some("1:1"));
        assert_eq!(inputs.quality.as_deref(), Some("high"));
        assert_eq!(inputs.reference_images, vec!["https://cdn.example/ref.png"]);
        assert_eq!(inputs.input_asset_ids, vec!["drive://space/asset-1"]);
    }

    /// The caller-selected model must reach the adapters, and it must be the
    /// vendor-stripped id: a `vendor/` prefix left on it would be sent upstream
    /// as part of the model name. Previously `from_command` hardcoded
    /// `model: String::new()`, so `model_or_default` always substituted the
    /// adapter default and `(!inputs.model.is_empty()).then(...)` dropped the
    /// field from the request body.
    #[test]
    fn command_inputs_carry_the_vendor_stripped_model() {
        let command = command_with_model("kling/kling-v2-master");
        let selection = resolve_vendor(&command, VENDOR_OPENAI);
        assert_eq!(selection.vendor, "kling");

        let inputs = GenerationCommandInputs::from_command(&command, &selection);
        assert_eq!(inputs.model, "kling-v2-master");
    }

    #[test]
    fn command_inputs_carry_the_bare_model_when_no_vendor_prefix_is_given() {
        let command = command_with_model("gpt-image-2");
        let selection = resolve_vendor(&command, VENDOR_OPENAI);

        let inputs = GenerationCommandInputs::from_command(&command, &selection);
        assert_eq!(inputs.model, "gpt-image-2");
    }

    fn command_with_model(model: &str) -> CreateGenerationCommandRequest {
        CreateGenerationCommandRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "a cat".to_string(),
            model: Some(model.to_string()),
            input_asset_ids: None,
            parameters: None,
        }
    }
}
