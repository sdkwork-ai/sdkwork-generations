//! Bounded settlement wait for generation retrieval tools.
//!
//! `get_generation` on the port refreshes async vendor tasks on read, so a
//! bounded poll loop is the natural wait primitive. Retrieval defaults to
//! waiting so a chat turn spends one tool call per generation instead of a
//! model-driven poll loop that burns tokens and drags out the turn.

use std::sync::Arc;
use std::time::{Duration, Instant};

use sdkwork_intelligence_generations_service::domain::models::{
    GenerationRecord, GenerationStatus,
};
use sdkwork_intelligence_generations_service::error::GenerationsError;

use crate::port::GenerationsMcpPort;

/// Wait budget applied when a retrieve tool omits `waitTimeoutSeconds`.
pub const DEFAULT_WAIT_SECONDS: f64 = 90.0;
/// Hard upper bound for the wait budget regardless of the requested value.
pub const MAX_WAIT_SECONDS: f64 = 120.0;
/// Interval between settlement polls (each read refreshes the vendor task).
pub const POLL_INTERVAL: Duration = Duration::from_millis(2_000);

/// Whether the status ends the wait: a final state, or a state a human must
/// act on (`requires_action` — waiting longer cannot progress the task).
pub fn status_settles(status: &GenerationStatus) -> bool {
    matches!(
        status,
        GenerationStatus::Succeeded
            | GenerationStatus::Failed
            | GenerationStatus::Canceled
            | GenerationStatus::RequiresAction
    )
}

/// Clamps the requested budget into `[1.0, MAX_WAIT_SECONDS]`; non-finite or
/// missing values fall back to [`DEFAULT_WAIT_SECONDS`].
pub fn clamp_wait_seconds(requested: Option<f64>) -> Duration {
    let seconds = match requested {
        Some(value) if value.is_finite() => value.clamp(1.0, MAX_WAIT_SECONDS),
        _ => DEFAULT_WAIT_SECONDS,
    };
    Duration::from_secs_f64(seconds)
}

/// Polls `get_generation` until the record settles or the budget expires.
///
/// The last read record is always returned — an expiring budget yields the
/// final in-flight snapshot, never an error, so the model can decide to
/// inform the user and check the generation history later.
pub async fn wait_for_settlement(
    port: &Arc<dyn GenerationsMcpPort>,
    generation_id: &str,
    budget: Duration,
) -> Result<GenerationRecord, GenerationsError> {
    let deadline = Instant::now() + budget;
    let mut record = port.get_generation(generation_id).await?;
    while !status_settles(&record.status) && Instant::now() < deadline {
        tokio::time::sleep(POLL_INTERVAL).await;
        record = port.get_generation(generation_id).await?;
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_and_action_statuses_settle() {
        assert!(status_settles(&GenerationStatus::Succeeded));
        assert!(status_settles(&GenerationStatus::Failed));
        assert!(status_settles(&GenerationStatus::Canceled));
        assert!(status_settles(&GenerationStatus::RequiresAction));
        assert!(!status_settles(&GenerationStatus::Queued));
        assert!(!status_settles(&GenerationStatus::Running));
    }

    #[test]
    fn wait_budget_defaults_and_clamps() {
        assert_eq!(clamp_wait_seconds(None), Duration::from_secs_f64(90.0));
        assert_eq!(
            clamp_wait_seconds(Some(30.0)),
            Duration::from_secs_f64(30.0)
        );
        assert_eq!(
            clamp_wait_seconds(Some(5_000.0)),
            Duration::from_secs_f64(120.0)
        );
        assert_eq!(
            clamp_wait_seconds(Some(0.0)),
            Duration::from_secs_f64(1.0)
        );
        assert_eq!(
            clamp_wait_seconds(Some(f64::NAN)),
            Duration::from_secs_f64(90.0)
        );
        assert_eq!(
            clamp_wait_seconds(Some(f64::INFINITY)),
            Duration::from_secs_f64(90.0)
        );
    }

    #[tokio::test]
    async fn wait_returns_the_settled_record_without_overshooting_the_budget() {
        use async_trait::async_trait;
        use sdkwork_intelligence_generations_service::domain::models::{
            CreateGenerationCommandRequest, GenerationModality, GenerationResult,
        };
        use sdkwork_intelligence_generations_service::ports::{
            ListResultsParams, ListTimelineParams,
        };
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct FlipAfterTwoReads {
            reads: AtomicUsize,
        }

        #[async_trait]
        impl GenerationsMcpPort for FlipAfterTwoReads {
            async fn create_generation(
                &self,
                _modality: GenerationModality,
                _operation_type: &str,
                _command: &CreateGenerationCommandRequest,
            ) -> Result<GenerationRecord, GenerationsError> {
                unreachable!("wait fixture never creates");
            }

            async fn get_generation(
                &self,
                _generation_id: &str,
            ) -> Result<GenerationRecord, GenerationsError> {
                let read = self.reads.fetch_add(1, Ordering::SeqCst);
                Ok(GenerationRecord {
                    id: "generation.1".to_string(),
                    tenant_id: "0".to_string(),
                    organization_id: None,
                    user_id: "wait-test".to_string(),
                    modality: GenerationModality::Image,
                    operation_type: "text_to_image".to_string(),
                    source_provider: None,
                    source_job_id: None,
                    prompt_preview: None,
                    status: if read >= 2 {
                        GenerationStatus::Succeeded
                    } else {
                        GenerationStatus::Running
                    },
                    favorite: false,
                    result_count: 0,
                    created_at: "2026-10-07T00:00:00Z".to_string(),
                    updated_at: "2026-10-07T00:00:00Z".to_string(),
                })
            }

            async fn list_results(
                &self,
                _generation_id: &str,
                _params: ListResultsParams,
            ) -> Result<(Vec<GenerationResult>, sdkwork_intelligence_generations_service::domain::models::PageInfo), GenerationsError>
            {
                unreachable!("wait fixture never lists results");
            }

            async fn list_timeline(
                &self,
                _generation_id: &str,
                _params: ListTimelineParams,
            ) -> Result<(Vec<sdkwork_intelligence_generations_service::domain::models::GenerationTimelineEvent>, sdkwork_intelligence_generations_service::domain::models::PageInfo), GenerationsError>
            {
                unreachable!("wait fixture never lists timeline");
            }
        }

        let port: Arc<dyn GenerationsMcpPort> = Arc::new(FlipAfterTwoReads {
            reads: AtomicUsize::new(0),
        });
        let record = wait_for_settlement(&port, "generation.1", Duration::from_secs(10))
            .await
            .expect("settlement wait succeeds");
        assert_eq!(record.status, GenerationStatus::Succeeded);
    }
}
