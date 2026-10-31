//! Generation pricing estimation port.
//!
//! The app surface owns the *interface* for "how many credits will this
//! generation command consume": this module defines the wire contract and the
//! [`GenerationPricingPort`] every host must serve behind
//! `POST /app/v3/api/generations/pricing/estimate`.
//!
//! The *implementation* belongs to the host that owns the pricing engine. In
//! the platform composition that host is `sdkwork-cloudrouter`, whose rate
//! book (`pricing_price_book` / `pricing_rate`) and token-bank conversion are
//! the same authority the invocation billing pipeline charges with. Estimating
//! anywhere else would drift from what the wallet is actually debited.

use serde::{Deserialize, Serialize};

use super::models::GenerationModality;

/// Caller scope the estimate is priced for.
///
/// The pricing catalog carries tenant/organization-scoped rates and pricing
/// plans; hosts resolve the caller's scope from the authenticated request
/// context and pass it through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationPricingScope {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub user_id: String,
}

/// Request for a pre-execution credit estimate.
///
/// Field parity with `CreateGenerationCommandRequest` is deliberate: the
/// estimate must price the command the client is about to submit, so the
/// client forwards the same `modality`/`operationType` (from its creation
/// dispatch), `model`, and `parameters` objects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationPricingEstimateRequest {
    pub modality: GenerationModality,
    /// Explicit operation. Omitted operations resolve through the same
    /// modality defaults the creation endpoints use.
    #[serde(rename = "operationType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_type: Option<String>,
    /// Model id the command would carry. Required for a priced estimate; a
    /// request without one answers `priced: false` instead of guessing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Free-form generation parameters, exactly as the command carries them
    /// (including the `generationConfig` block the creative composer builds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// Prompt the command would carry. Only its length is read, for
    /// character-metered modalities (voice/speech); the text itself never
    /// leaves the pricing evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// One priced meter line of an estimate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationPricingEstimateLine {
    /// Billing meter code the line was priced against (e.g.
    /// `video_output_second`, `image_result`).
    #[serde(rename = "meterCode")]
    pub meter_code: String,
    /// Quantity the charge will scale with (declared duration, image count,
    /// prompt characters, ...).
    pub quantity: String,
    /// Where the quantity came from: `requested` when the client declared it,
    /// `single_unit` when the request carries no derivable quantity so one
    /// unit is shown as a lower bound.
    #[serde(rename = "quantityBasis")]
    pub quantity_basis: String,
    /// Unit the quantity is denominated in (`second`, `character`, `item`).
    #[serde(rename = "quantityUnit")]
    pub quantity_unit: String,
    /// Customer-charge unit price for the meter, as a decimal string.
    #[serde(rename = "unitPrice")]
    pub unit_price: String,
    /// Pricing currency of `unit_price` (ISO 4217).
    #[serde(rename = "currencyCode")]
    pub currency_code: String,
    /// Divisor applied to `unit_price` per unit (e.g. `1000000` for
    /// per-million token meters, `60` for per-minute audio meters).
    #[serde(rename = "unitSize")]
    pub unit_size: String,
    /// Cash amount for this line (`unit_price × quantity / unit_size`),
    /// decimal string in `currencyCode`.
    pub amount: String,
    /// Token-bank points for this line, as a decimal string in points
    /// (`points_micros` / 1e6).
    pub points: String,
    /// Exact integer micro-points (1 point = 1e6 micro) the wallet arithmetic
    /// uses, so clients can compare against a micro-denominated balance.
    #[serde(rename = "pointsMicros")]
    pub points_micros: i64,
    /// Pricing tier the rate was conditioned on (video resolution tiers,
    /// image quality tiers, ...), when the catalog declared one.
    #[serde(rename = "tierCode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier_code: Option<String>,
}

/// Pre-execution credit estimate for one generation command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationPricingEstimate {
    /// `true` when every chargeable meter resolved a rate and `points_total`
    /// is the real projected charge. `false` answers carry `unpriced_reason`.
    pub priced: bool,
    /// Total points for the command, decimal string in points.
    #[serde(rename = "pointsTotal")]
    pub points_total: String,
    /// Exact total in micro-points.
    #[serde(rename = "pointsMicros")]
    pub points_micros: i64,
    /// Pricing currency the lines were charged in, when priced.
    #[serde(rename = "currencyCode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_code: Option<String>,
    /// Model the estimate was priced against (echo of the request).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Pricing catalog key the model resolved to (`vendor/model`).
    #[serde(rename = "catalogKey")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_key: Option<String>,
    /// Vendor segment of the resolved catalog key.
    #[serde(rename = "vendorCode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_code: Option<String>,
    /// Billing region the rate was resolved in.
    #[serde(rename = "regionCode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_code: Option<String>,
    /// Per-meter breakdown, ordered by the catalog's primary-meter preference.
    pub lines: Vec<GenerationPricingEstimateLine>,
    /// Diagnosis when `priced` is false (model not found, no rate for the
    /// requested dimensions, ambiguous model name, ...).
    #[serde(rename = "unpricedReason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unpriced_reason: Option<String>,
    /// When the estimate was computed (RFC 3339).
    #[serde(rename = "quotedAt")]
    pub quoted_at: String,
}

/// Errors the estimate port reports.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GenerationPricingError {
    /// The host has not wired a pricing implementation. The endpoint answers
    /// 501 so clients can fall back to "unavailable" instead of a fake number.
    #[error("generation pricing estimation is not configured on this deployment")]
    NotConfigured,
    /// The pricing engine rejected the request (scope unresolvable, catalog
    /// unavailable, ...). The endpoint answers 503 with this message.
    #[error("generation pricing estimation failed: {0}")]
    Backend(String),
}

/// The estimation seam every host serving the generations app surface must
/// implement (or leave unconfigured and answer 501).
#[cfg_attr(feature = "http-axum", async_trait::async_trait)]
pub trait GenerationPricingPort: Send + Sync {
    /// Estimate the credit consumption of one generation command.
    async fn estimate(
        &self,
        request: &GenerationPricingEstimateRequest,
        scope: &GenerationPricingScope,
    ) -> Result<GenerationPricingEstimate, GenerationPricingError>;
}

/// Default port for hosts without a pricing engine: every estimate reports
/// `NotConfigured` so the HTTP surface fails loud instead of inventing a cost.
pub struct UnconfiguredGenerationPricingPort;

#[cfg_attr(feature = "http-axum", async_trait::async_trait)]
impl GenerationPricingPort for UnconfiguredGenerationPricingPort {
    async fn estimate(
        &self,
        _request: &GenerationPricingEstimateRequest,
        _scope: &GenerationPricingScope,
    ) -> Result<GenerationPricingEstimate, GenerationPricingError> {
        Err(GenerationPricingError::NotConfigured)
    }
}

/// Operation defaults per modality, mirroring the creation dispatch the client
/// applies before submitting (`text_to_image` with references upgrades to
/// `image_edit`, and so on). Operations the client resolved explicitly ride on
/// the request and win over these defaults.
pub fn default_operation_for_modality(modality: &GenerationModality) -> &'static str {
    match modality {
        GenerationModality::Image => "text_to_image",
        GenerationModality::Video => "text_to_video",
        GenerationModality::Music => "text_to_music",
        GenerationModality::Audio => "transcription",
        GenerationModality::Sfx => "sound_effects",
        GenerationModality::Voice => "speech",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(modality: GenerationModality) -> GenerationPricingEstimateRequest {
        GenerationPricingEstimateRequest {
            modality,
            operation_type: None,
            model: Some("2.0-mini".to_owned()),
            parameters: None,
            prompt: None,
        }
    }

    #[test]
    fn request_serializes_with_camel_case_contract_fields() {
        let value = serde_json::to_value(request(GenerationModality::Video)).unwrap();
        assert_eq!(value["modality"], "video");
        assert_eq!(value["model"], "2.0-mini");
        assert!(value.get("operationType").is_none());
        assert!(value.get("prompt").is_none());
    }

    #[test]
    fn request_parses_generation_config_parameters() {
        let value = serde_json::json!({
            "modality": "video",
            "operationType": "text_to_video",
            "model": "2.0-mini",
            "parameters": { "generationConfig": { "resolution": "1080P" } },
            "prompt": "a cat"
        });
        let parsed: GenerationPricingEstimateRequest = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.operation_type.as_deref(), Some("text_to_video"));
        assert_eq!(parsed.prompt.as_deref(), Some("a cat"));
        assert_eq!(
            parsed.parameters.unwrap()["generationConfig"]["resolution"],
            "1080P"
        );
    }

    #[test]
    fn estimate_response_round_trips_lines_and_reason() {
        let estimate = GenerationPricingEstimate {
            priced: false,
            points_total: "0".to_owned(),
            points_micros: 0,
            currency_code: None,
            model: Some("2.0-mini".to_owned()),
            catalog_key: None,
            vendor_code: None,
            region_code: None,
            lines: vec![],
            unpriced_reason: Some("model not found".to_owned()),
            quoted_at: "2026-10-31T00:00:00Z".to_owned(),
        };
        let value = serde_json::to_value(&estimate).unwrap();
        assert_eq!(value["priced"], false);
        assert_eq!(value["unpricedReason"], "model not found");
        assert_eq!(value["pointsMicros"], 0);
        let parsed: GenerationPricingEstimate = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, estimate);
    }

    #[test]
    fn unconfigured_port_reports_not_configured() {
        let port = UnconfiguredGenerationPricingPort;
        let scope = GenerationPricingScope {
            tenant_id: "1".to_owned(),
            organization_id: None,
            user_id: "u".to_owned(),
        };
        let error = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(port.estimate(&request(GenerationModality::Image), &scope))
            .unwrap_err();
        assert!(matches!(error, GenerationPricingError::NotConfigured));
    }

    #[test]
    fn modality_defaults_match_creation_dispatch() {
        assert_eq!(
            default_operation_for_modality(&GenerationModality::Image),
            "text_to_image"
        );
        assert_eq!(
            default_operation_for_modality(&GenerationModality::Sfx),
            "sound_effects"
        );
        assert_eq!(
            default_operation_for_modality(&GenerationModality::Voice),
            "speech"
        );
    }
}
