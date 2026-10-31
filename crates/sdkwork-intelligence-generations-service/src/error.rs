//! Error types for the generations service.

use thiserror::Error;

/// Errors that can occur within the generations bounded context.
#[derive(Debug, Error)]
pub enum GenerationsError {
    /// Requested resource was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Persistence layer error.
    #[error("storage error: {0}")]
    Storage(String),

    /// Invalid input from the caller.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// External provider error.
    #[error("provider error: {0}")]
    Provider(String),

    /// Asset catalog integration error.
    #[error("asset error: {0}")]
    Asset(String),

    /// The generation cannot be transitioned to the requested state.
    #[error("conflict: {0}")]
    Conflict(String),

    /// Rate limit or quota exceeded.
    #[error("rate limited: {0}")]
    RateLimited(String),

    /// Credit estimation is not wired on this deployment. The pricing port
    /// lives in the composition host (cloudrouter); a missing implementation
    /// is a deployment misconfiguration, reported as 501.
    #[error("generation pricing estimation is not configured on this deployment")]
    PricingNotConfigured,

    /// The pricing engine rejected an estimation request.
    #[error("pricing error: {0}")]
    Pricing(String),

    /// Unclassified internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl GenerationsError {
    /// Return the HTTP status code appropriate for this error.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::Storage(_) => 500,
            Self::InvalidInput(_) => 400,
            Self::Provider(_) => 502,
            Self::Asset(_) => 502,
            Self::Conflict(_) => 409,
            Self::RateLimited(_) => 429,
            Self::PricingNotConfigured => 501,
            Self::Pricing(_) => 503,
            Self::Internal(_) => 500,
        }
    }

    /// Return the platform error code for API error envelopes.
    pub fn platform_code(&self) -> i32 {
        match self {
            Self::NotFound(_) => 40401,
            Self::Storage(_) => 50001,
            Self::InvalidInput(_) => 40001,
            Self::Provider(_) => 50201,
            Self::Asset(_) => 50202,
            Self::Conflict(_) => 40901,
            Self::RateLimited(_) => 42901,
            Self::PricingNotConfigured => 50101,
            Self::Pricing(_) => 50301,
            Self::Internal(_) => 50001,
        }
    }
}

impl From<sqlx::Error> for GenerationsError {
    fn from(err: sqlx::Error) -> Self {
        Self::Storage(err.to_string())
    }
}

impl From<serde_json::Error> for GenerationsError {
    fn from(err: serde_json::Error) -> Self {
        Self::Internal(err.to_string())
    }
}
