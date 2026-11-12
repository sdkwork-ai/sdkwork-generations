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
        let vendor = normalize_vendor(vendor);
        // An explicit vendor wins, but a redundant `vendor/model` prefix on the
        // model id must still be stripped — the wire model is documented as the
        // vendor-stripped id, and `vidu/viduq2` sent verbatim would be rejected
        // by the upstream as an unknown model.
        let model = match raw_model.split_once('/') {
            Some((prefix, rest))
                if !prefix.trim().is_empty()
                    && !rest.trim().is_empty()
                    && normalize_vendor(prefix) == vendor =>
            {
                rest.trim().to_string()
            }
            _ => raw_model.to_string(),
        };
        return VendorSelection { vendor, model };
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
        // The model catalog files Kling under its parent company code
        // (`vendorCode: "kuaishou"`), while every dispatch surface — and the
        // creative pickers that forward that code as `parameters.vendor` —
        // must land on the Kling adapter.
        "kuaishou" => VENDOR_KLING.to_string(),
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
    /// Whether the vendor should return synchronized audio (video) when the
    /// model supports it.
    pub audio: Option<bool>,
    /// Precise generation mode the creative picker resolved from the catalog
    /// model profile (`reference_to_video`, `start_end_frame`, ...).
    pub generation_mode: Option<String>,
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
            generation_mode: string_from(&["generationMode", "generation_mode"]),
            audio: bool_from(&["audio", "generateAudio", "generate_audio"]),
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

    /// Vidu resolution token for the composer's resolution tier.
    ///
    /// The composer offers `1K` / `2K` / `4K` image tiers, while Vidu's wire
    /// values are `1080p` / `2K` / `4K`; video tiers already use the wire form
    /// and pass through lowercased.
    pub fn vidu_resolution(&self) -> Option<String> {
        match self
            .resolution
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("1k") | Some("1080p") => Some("1080p".to_string()),
            Some("2k") => Some("2K".to_string()),
            Some("4k") => Some("4K".to_string()),
            Some("480p") => Some("480p".to_string()),
            Some("540p") => Some("540p".to_string()),
            Some("720p") => Some("720p".to_string()),
            other => other.map(str::to_string),
        }
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

/// Vendor-accepted video duration vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoDurationRule {
    /// The vendor accepts only these whole-second values.
    Discrete(&'static [i64]),
    /// The vendor accepts any whole second inside the inclusive window.
    Range { min: i64, max: i64 },
}

/// Official duration rule for a wired video surface.
///
/// Retrieved from the vendor API references (2026-11):
/// * OpenAI Videos: `seconds` is 4/8/12; extensions accept 4/8/12/16/20.
///   <https://developers.openai.com/api/reference/resources/videos>
/// * Google Veo 3.x: `durationSeconds` is 4/6/8.
///   <https://ai.google.dev/gemini-api/docs/veo>
/// * Kling 3.x: 3–15 s. <https://kling.ai/document-api/guides/capability-map/video.md>
/// * Vidu Q3/Q4: 1–16 s. <https://platform.vidu.com/docs/api-reference/video-models/vidu-q3/text-to-video>
/// * Seedance: 2–30 s (2.5 extends the 2.0 window).
pub fn video_duration_rule(
    vendor: &str,
    model: &str,
    operation: &str,
) -> Option<VideoDurationRule> {
    match vendor {
        "openai" if operation == "video_extend" => {
            Some(VideoDurationRule::Discrete(&[4, 8, 12, 16, 20]))
        }
        "openai" => Some(VideoDurationRule::Discrete(&[4, 8, 12])),
        "google" if model.starts_with("veo-3") => {
            Some(VideoDurationRule::Discrete(&[4, 6, 8]))
        }
        "google" if model.starts_with("veo-") => Some(VideoDurationRule::Range { min: 5, max: 8 }),
        "kuaishou" => Some(VideoDurationRule::Range { min: 3, max: 15 }),
        "vidu" => Some(VideoDurationRule::Range { min: 1, max: 16 }),
        "bytedance" => Some(VideoDurationRule::Range { min: 2, max: 30 }),
        _ => None,
    }
}

/// Aspect ratios the vendor documents for a video surface (`None` = no local
/// restriction).
pub fn video_aspect_ratios(vendor: &str, model: &str) -> Option<&'static [&'static str]> {
    match vendor {
        "openai" => Some(&["16:9", "9:16"]),
        "google" if model.starts_with("veo-3") => Some(&["16:9", "9:16"]),
        "google" => Some(&["16:9", "9:16", "1:1"]),
        "kuaishou" | "vidu" => Some(&["16:9", "9:16", "1:1"]),
        "bytedance" => Some(&["16:9", "4:3", "1:1", "3:4", "9:16", "21:9"]),
        _ => None,
    }
}

fn format_duration_rule(rule: VideoDurationRule) -> String {
    match rule {
        VideoDurationRule::Discrete(values) => values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join("/"),
        VideoDurationRule::Range { min, max } => format!("{min}-{max}"),
    }
}

/// Validate a requested video duration against the vendor's published rule.
///
/// Fails locally with an actionable message instead of forwarding a value the
/// vendor is documented to reject (which otherwise surfaces as an opaque 400
/// after the task is already recorded).
pub fn validate_video_duration(
    vendor: &str,
    model: &str,
    operation: &str,
    inputs: &GenerationCommandInputs,
) -> Result<(), String> {
    let Some(value) = inputs.duration_seconds else {
        return Ok(());
    };
    let Some(rule) = video_duration_rule(vendor, model, operation) else {
        return Ok(());
    };
    if !value.is_finite() || value <= 0.0 {
        return Err(format!(
            "durationSeconds must be a positive number of seconds; received {value}"
        ));
    }
    if value.fract() != 0.0 {
        return Err(format!(
            "durationSeconds must be whole seconds for {vendor}/{model}; received {value}"
        ));
    }
    let seconds = value as i64;
    match rule {
        VideoDurationRule::Discrete(values) if !values.contains(&seconds) => Err(format!(
            "{vendor}/{model} supports durationSeconds {}; received {seconds}",
            format_duration_rule(rule)
        )),
        VideoDurationRule::Range { min, max } if seconds < min || seconds > max => Err(format!(
            "{vendor}/{model} supports durationSeconds {min}-{max}; received {seconds}"
        )),
        _ => Ok(()),
    }
}

/// Validate a requested video aspect ratio against the vendor's published set.
pub fn validate_video_aspect_ratio(
    vendor: &str,
    model: &str,
    inputs: &GenerationCommandInputs,
) -> Result<(), String> {
    let Some(ratio) = inputs
        .aspect_ratio
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let Some(supported) = video_aspect_ratios(vendor, model) else {
        return Ok(());
    };
    if supported.contains(&ratio) {
        return Ok(());
    }
    Err(format!(
        "{vendor}/{model} supports aspect ratios {}; received {ratio}",
        supported.join("/")
    ))
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
    fn resolves_catalog_vendor_code_kuaishou_to_the_kling_adapter() {
        // The creative pickers submit the bare model id from the model catalog
        // (`vendorCode: "kuaishou"`) together with `parameters.vendor`; without
        // this alias the command falls back to the OpenAI default vendor and
        // the Kling model id is sent to the wrong upstream.
        let mut command = command_with_model("kling-v2-6");
        command.parameters = Some(serde_json::json!({ "vendor": "kuaishou" }));
        let selection = resolve_vendor(&command, "openai");
        assert_eq!(selection.vendor, VENDOR_KLING);
        assert_eq!(selection.model, "kling-v2-6");

        let selection_from_prefix = resolve_vendor(&command_with_model("kuaishou/kling-v2-6"), "openai");
        assert_eq!(selection_from_prefix.vendor, VENDOR_KLING);
        assert_eq!(selection_from_prefix.model, "kling-v2-6");
    }

    #[test]
    fn explicit_vendor_still_strips_a_redundant_matching_model_prefix() {
        // `parameters.vendor` wins over the model prefix, but the wire model
        // stays the vendor-stripped id: `vidu/viduq2` sent verbatim would be
        // rejected by the upstream as an unknown model.
        let mut command = command_with_model("vidu/viduq2");
        command.parameters = Some(serde_json::json!({ "vendor": "vidu" }));
        let selection = resolve_vendor(&command, "openai");
        assert_eq!(selection.vendor, "vidu");
        assert_eq!(selection.model, "viduq2");

        // A prefix that normalizes to a DIFFERENT vendor than the explicit one
        // is kept verbatim — the caller's model id is authoritative and the
        // conflicting prefix is not ours to rewrite.
        let mut conflicting = command_with_model("kling/viduq2");
        conflicting.parameters = Some(serde_json::json!({ "vendor": "vidu" }));
        let selection = resolve_vendor(&conflicting, "openai");
        assert_eq!(selection.vendor, "vidu");
        assert_eq!(selection.model, "kling/viduq2");
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

    #[test]
    fn vidu_resolution_maps_composer_tiers_to_wire_tokens() {
        let resolve = |resolution: &str| {
            let mut command = command_with_model("vidu/viduq2");
            command.parameters = Some(serde_json::json!({
                "generationConfig": { "resolution": resolution }
            }));
            let selection = resolve_vendor(&command, "vidu");
            GenerationCommandInputs::from_command(&command, &selection).vidu_resolution()
        };
        assert_eq!(resolve("1K").as_deref(), Some("1080p"));
        assert_eq!(resolve("2K").as_deref(), Some("2K"));
        assert_eq!(resolve("4K").as_deref(), Some("4K"));
        assert_eq!(resolve("720P").as_deref(), Some("720p"));
        assert_eq!(resolve("1080P").as_deref(), Some("1080p"));
    }

    #[test]
    fn command_inputs_extract_the_generation_mode() {
        let mut command = command_with_model("vidu/viduq3");
        command.parameters = Some(serde_json::json!({ "generationMode": "reference_to_video" }));
        let selection = resolve_vendor(&command, "vidu");
        let inputs = GenerationCommandInputs::from_command(&command, &selection);
        assert_eq!(inputs.generation_mode.as_deref(), Some("reference_to_video"));
    }

    /// Every catalog vendor code the creative picker can emit must resolve to
    /// the adapter surface that dispatches it, and the wire model must be the
    /// vendor-stripped id.
    #[test]
    fn catalog_vendor_codes_map_to_their_adapter_surfaces() {
        let table = [
            ("openai", "openai"),
            ("google", "nano-banana"),
            ("kuaishou", "kling"),
            ("vidu", "vidu"),
            ("bytedance", "volcengine"),
            ("suno", "suno"),
            ("minimax", "minimax"),
            ("elevenlabs", "elevenlabs"),
        ];
        for (catalog_code, adapter_vendor) in table {
            let mut command = command_with_model(&format!("{catalog_code}/catalog-model"));
            command.parameters = Some(serde_json::json!({ "vendor": catalog_code }));
            let selection = resolve_vendor(&command, "openai");
            assert_eq!(
                selection.vendor, adapter_vendor,
                "catalog vendor {catalog_code} must dispatch through {adapter_vendor}"
            );
            assert_eq!(
                selection.model, "catalog-model",
                "the explicit vendor strips the redundant prefix"
            );
        }
    }

    #[test]
    fn video_duration_rules_match_the_vendor_documentation() {
        let sora = inputs_with_duration(Some(8.0), None);
        assert!(validate_video_duration("openai", "sora-2", "text_to_video", &sora).is_ok());
        let sora_odd = inputs_with_duration(Some(10.0), None);
        assert!(validate_video_duration("openai", "sora-2", "text_to_video", &sora_odd)
            .unwrap_err()
            .contains("4/8/12"));
        let extend = inputs_with_duration(Some(16.0), None);
        assert!(
            validate_video_duration("openai", "sora-2", "video_extend", &extend).is_ok(),
            "extensions accept 16 and 20 seconds"
        );

        let veo5 = inputs_with_duration(Some(5.0), None);
        assert!(validate_video_duration("google", "veo-3.1-generate-preview", "text_to_video", &veo5)
            .is_err());
        let veo6 = inputs_with_duration(Some(6.0), None);
        assert!(validate_video_duration("google", "veo-3.1-generate-preview", "text_to_video", &veo6)
            .is_ok());

        let kling20 = inputs_with_duration(Some(20.0), None);
        assert!(validate_video_duration("kuaishou", "kling-v3", "text_to_video", &kling20).is_err());
        let vidu16 = inputs_with_duration(Some(16.0), None);
        assert!(validate_video_duration("vidu", "viduq3-pro", "text_to_video", &vidu16).is_ok());
        let seedance31 = inputs_with_duration(Some(31.0), None);
        assert!(
            validate_video_duration("bytedance", "doubao-seedance-2-5-260623", "text_to_video", &seedance31)
                .is_err()
        );
        let fractional = inputs_with_duration(Some(5.5), None);
        assert!(validate_video_duration("kuaishou", "kling-v3", "text_to_video", &fractional)
            .unwrap_err()
            .contains("whole seconds"));
    }

    #[test]
    fn video_ratio_rules_match_the_vendor_documentation() {
        let openai_square = inputs_with_duration(None, Some("1:1"));
        assert!(validate_video_aspect_ratio("openai", "sora-2", &openai_square).is_err());
        let openai_wide = inputs_with_duration(None, Some("16:9"));
        assert!(validate_video_aspect_ratio("openai", "sora-2", &openai_wide).is_ok());

        let veo_square = inputs_with_duration(None, Some("1:1"));
        assert!(
            validate_video_aspect_ratio("google", "veo-3.1-generate-preview", &veo_square).is_err()
        );

        let kling_square = inputs_with_duration(None, Some("1:1"));
        assert!(validate_video_aspect_ratio("kuaishou", "kling-v3", &kling_square).is_ok());

        let seedance_wide = inputs_with_duration(None, Some("21:9"));
        assert!(
            validate_video_aspect_ratio("bytedance", "doubao-seedance-2-0-260128", &seedance_wide)
                .is_ok()
        );
        let seedance_unknown = inputs_with_duration(None, Some("5:4"));
        assert!(
            validate_video_aspect_ratio("bytedance", "doubao-seedance-2-0-260128", &seedance_unknown)
                .is_err()
        );
        // An unconstrained vendor passes anything through.
        assert!(validate_video_aspect_ratio("suno", "suno-v6", &seedance_unknown).is_ok());
    }

    fn inputs_with_duration(
        duration_seconds: Option<f64>,
        aspect_ratio: Option<&str>,
    ) -> GenerationCommandInputs {
        GenerationCommandInputs {
            prompt: "a cat".to_string(),
            model: "catalog-model".to_string(),
            image_count: None,
            size: None,
            quality: None,
            response_format: None,
            aspect_ratio: aspect_ratio.map(str::to_string),
            resolution: None,
            duration_seconds,
            seed: None,
            negative_prompt: None,
            voice: None,
            voice_id: None,
            speed: None,
            stability: None,
            similarity: None,
            style: None,
            language: None,
            emotion: None,
            is_instrumental: None,
            lyrics_optimizer: None,
            sample_rate: None,
            audio_bitrate: None,
            audio_format: None,
            tags: None,
            title: None,
            lyrics: None,
            negative_tags: None,
            mode: None,
            cfg_scale: None,
            loop_enabled: None,
            audio: None,
            generation_mode: None,
            reference_images: Vec::new(),
            reference_image_tail: None,
            reference_videos: Vec::new(),
            audio_url: None,
            audio_text: None,
            input_asset_ids: Vec::new(),
        }
    }

    fn command_with_model(model: &str) -> CreateGenerationCommandRequest {
        CreateGenerationCommandRequest {
            conversation_id: None,
            tenant_id: "tenant-1".to_string(),
            organization_id: None,
            prompt: "a cat".to_string(),
            model: Some(model.to_string()),
            input_asset_ids: None,
            parameters: None,
        }
    }
}
