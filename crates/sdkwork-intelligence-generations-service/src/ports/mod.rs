//! Port interfaces for the generations domain.
//!
//! Ports define the boundaries between the business kernel and external
//! infrastructure (persistence, AI providers, asset catalog). Concrete
//! implementations are injected at the assembly layer.

pub mod provider;
pub mod repository;

pub use provider::{
    AssetPort, GenerationDispatchOutcome, GenerationProvider, GenerationUsage,
    GenerationUsageFact, GenerationUsagePort, GenerationUsageSource,
};
pub use repository::{
    CreateGenerationParams, GenerationRepository, GenerationResultRepository,
    ListGenerationsParams, ListResultsParams, ListTimelineParams, TimelineRepository,
    UpdateGenerationProviderStateParams,
};
