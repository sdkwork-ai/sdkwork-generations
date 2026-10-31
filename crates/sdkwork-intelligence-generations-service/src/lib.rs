//! Business service crate for sdkwork-generations.
//!
//! Owns domain models, commands, results, provider ports, repository ports,
//! and the generation service use cases that power the HTTP surfaces.

mod config;
pub mod context;
pub mod domain;
pub mod error;
pub mod ports;
mod service;

pub use config::{GenerationsConfig, SharedGenerationsConfig};
pub use context::{GenerationsHttpRequestContext, GenerationsRequestContext};
pub use domain::models::{
    CreateGenerationCommandRequest, FavoriteGenerationRequest, GenerationActionRequest,
    GenerationCommandResponse, GenerationModality, GenerationRecord, GenerationRecordPage,
    GenerationResult, GenerationResultPage, GenerationStatus, GenerationTimelineEvent,
    GenerationTimelineEventPage, MediaResource, PageInfo, SaveGenerationResultToAssetsRequest,
};
pub use domain::pricing::{
    default_operation_for_modality, GenerationPricingError, GenerationPricingEstimate,
    GenerationPricingEstimateLine, GenerationPricingEstimateRequest, GenerationPricingPort,
    GenerationPricingScope, UnconfiguredGenerationPricingPort,
};
pub use error::GenerationsError;
pub use service::{GenerationsService, GenerationsServiceState, build_app_routes};

/// HTTP state alias shared across surfaces.
pub type GenerationsHttpState = GenerationsServiceState;
