//! Image and video generation lifecycle regression tests.
//!
//! These exercise the service create → dispatch → persist → refresh chain
//! against in-memory ports and scripted providers, pinning the contract that
//! async vendor tasks (Kling, Vidu, Volcengine, Veo, nano-banana) reach a
//! terminal state through the refresh path and that the resolved vendor
//! persisted at dispatch routes the polling.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::config::GenerationsConfig;
use crate::context::GenerationsRequestContext;
use crate::domain::models::{
    CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationResult,
    GenerationStatus, GenerationTimelineEvent, SaveGenerationResultToAssetsRequest,
};
use crate::error::GenerationsError;
use crate::ports::{
    AssetPort, CreateGenerationParams, GenerationDispatchOutcome, GenerationProvider,
    GenerationRepository, GenerationResultRepository, GenerationUsageFact, GenerationUsagePort,
    ListGenerationsParams, ListResultsParams, ListTimelineParams, TimelineRepository,
    UpdateGenerationProviderStateParams,
};
use crate::service::GenerationsService;

// ---------------------------------------------------------------------------
// In-memory ports
// ---------------------------------------------------------------------------

#[derive(Default)]
struct InMemoryGenerationRepository {
    records: Mutex<std::collections::HashMap<String, GenerationRecord>>,
    sequence: Mutex<i64>,
}

impl InMemoryGenerationRepository {
    fn next_id(&self) -> String {
        let mut sequence = self.sequence.lock().unwrap();
        *sequence += 1;
        format!("gen-{sequence}")
    }
}

#[async_trait]
impl GenerationRepository for InMemoryGenerationRepository {
    async fn create(
        &self,
        params: CreateGenerationParams,
    ) -> Result<GenerationRecord, GenerationsError> {
        let id = self.next_id();
        let record = GenerationRecord {
            id: id.clone(),
            tenant_id: params.tenant_id,
            organization_id: params.organization_id,
            user_id: params.user_id,
            modality: GenerationModality::parse(&params.modality)
                .unwrap_or(GenerationModality::Image),
            operation_type: params.operation_type,
            source_provider: params.source_provider,
            source_job_id: params.source_job_id,
            prompt_preview: params.prompt_preview,
            status: GenerationStatus::Queued,
            favorite: false,
            result_count: 0,
            created_at: "2026-09-12T00:00:00+00:00".to_string(),
            updated_at: "2026-09-12T00:00:00+00:00".to_string(),
        };
        self.records.lock().unwrap().insert(id, record.clone());
        Ok(record)
    }

    async fn get(&self, id: &str) -> Result<Option<GenerationRecord>, GenerationsError> {
        Ok(self.records.lock().unwrap().get(id).cloned())
    }

    async fn list(
        &self,
        params: ListGenerationsParams,
    ) -> Result<(Vec<GenerationRecord>, Option<String>, bool), GenerationsError> {
        let records = self.records.lock().unwrap();
        let mut items: Vec<GenerationRecord> = records
            .values()
            .filter(|record| record.tenant_id == params.tenant_id)
            .cloned()
            .collect();
        items.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok((items, None, false))
    }

    async fn cancel(
        &self,
        id: &str,
        _reason: Option<&str>,
    ) -> Result<Option<GenerationRecord>, GenerationsError> {
        if let Some(record) = self.records.lock().unwrap().get_mut(id) {
            record.status = GenerationStatus::Canceled;
            return Ok(Some(record.clone()));
        }
        Ok(None)
    }

    async fn retry(
        &self,
        id: &str,
        _reason: Option<&str>,
    ) -> Result<Option<GenerationRecord>, GenerationsError> {
        Ok(self.records.lock().unwrap().get(id).cloned())
    }

    async fn set_favorite(
        &self,
        id: &str,
        favorite: bool,
    ) -> Result<Option<GenerationRecord>, GenerationsError> {
        if let Some(record) = self.records.lock().unwrap().get_mut(id) {
            record.favorite = favorite;
            return Ok(Some(record.clone()));
        }
        Ok(None)
    }

    async fn update_provider_state(
        &self,
        params: UpdateGenerationProviderStateParams,
    ) -> Result<Option<GenerationRecord>, GenerationsError> {
        let mut records = self.records.lock().unwrap();
        let Some(record) = records.get_mut(&params.id) else {
            return Ok(None);
        };
        if let Some(status) = params.status.as_deref() {
            if let Some(status) = GenerationStatus::parse(status) {
                record.status = status;
            }
        }
        if params.source_job_id.is_some() {
            record.source_job_id = params.source_job_id.clone();
        }
        if params.source_provider.is_some() {
            record.source_provider = params.source_provider.clone();
        }
        if let Some(result_count) = params.result_count {
            record.result_count = result_count;
        }
        Ok(Some(record.clone()))
    }
}

#[derive(Default)]
struct InMemoryResultRepository {
    results: Mutex<Vec<GenerationResult>>,
}

#[async_trait]
impl GenerationResultRepository for InMemoryResultRepository {
    async fn create(
        &self,
        result: &GenerationResult,
    ) -> Result<GenerationResult, GenerationsError> {
        self.results.lock().unwrap().push(result.clone());
        Ok(result.clone())
    }

    async fn get(
        &self,
        generation_id: &str,
        result_id: &str,
    ) -> Result<Option<GenerationResult>, GenerationsError> {
        Ok(self
            .results
            .lock()
            .unwrap()
            .iter()
            .find(|result| result.generation_id == generation_id && result.id == result_id)
            .cloned())
    }

    async fn list(
        &self,
        params: ListResultsParams,
    ) -> Result<(Vec<GenerationResult>, Option<String>, bool), GenerationsError> {
        let items = self
            .results
            .lock()
            .unwrap()
            .iter()
            .filter(|result| result.generation_id == params.generation_id)
            .cloned()
            .collect();
        Ok((items, None, false))
    }

    async fn update(
        &self,
        result: &GenerationResult,
    ) -> Result<GenerationResult, GenerationsError> {
        Ok(result.clone())
    }
}

#[derive(Default)]
struct InMemoryTimelineRepository {
    events: Mutex<Vec<GenerationTimelineEvent>>,
}

#[async_trait]
impl TimelineRepository for InMemoryTimelineRepository {
    async fn append(
        &self,
        event: &GenerationTimelineEvent,
    ) -> Result<GenerationTimelineEvent, GenerationsError> {
        self.events.lock().unwrap().push(event.clone());
        Ok(event.clone())
    }

    async fn list(
        &self,
        params: ListTimelineParams,
    ) -> Result<(Vec<GenerationTimelineEvent>, Option<String>, bool), GenerationsError> {
        let items = self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.generation_id == params.generation_id)
            .cloned()
            .collect();
        Ok((items, None, false))
    }
}

#[derive(Default)]
struct RecordingUsagePort {
    facts: Mutex<Vec<GenerationUsageFact>>,
}

#[async_trait]
impl GenerationUsagePort for RecordingUsagePort {
    async fn record_usage(&self, fact: &GenerationUsageFact) -> Result<(), GenerationsError> {
        self.facts.lock().unwrap().push(fact.clone());
        Ok(())
    }
}

struct NoopAssetPort;

#[async_trait]
impl AssetPort for NoopAssetPort {
    async fn save_generation_result(
        &self,
        _generation_id: &str,
        _result_id: &str,
        _request: &SaveGenerationResultToAssetsRequest,
        _context: &GenerationsRequestContext,
    ) -> Result<GenerationResult, GenerationsError> {
        Err(GenerationsError::Provider("asset port unused".to_string()))
    }
}

// ---------------------------------------------------------------------------
// Scripted provider
// ---------------------------------------------------------------------------

/// Dispatch behaviors a test can script. Every behavior builds its outcome
/// from the record the service passes in, mirroring how the real adapters
/// carry the resolved vendor and task id onto the record.
enum DispatchBehavior {
    /// Pend on a vendor task, persisting the resolved vendor like the real
    /// adapters do via `with_resolved_vendor`.
    PendOnTask { vendor: &'static str, task_id: &'static str },
    /// Complete immediately with a single image result.
    CompleteWithImage { url: &'static str },
    /// Fail the dispatch with a provider error.
    Fail { message: &'static str },
}

enum RetrievalBehavior {
    CompleteWithVideo { url: &'static str },
    CompleteWithImage { url: &'static str },
    StillRunning,
}

#[derive(Clone)]
struct ScriptedProvider {
    inner: Arc<ScriptedProviderInner>,
}

struct ScriptedProviderInner {
    provider_modality: GenerationModality,
    provider_vendor: &'static str,
    dispatch_queue: Mutex<Vec<DispatchBehavior>>,
    retrieval_queue: Mutex<Vec<RetrievalBehavior>>,
    retrieved_records: Mutex<Vec<GenerationRecord>>,
}

impl ScriptedProvider {
    fn new(
        provider_modality: GenerationModality,
        provider_vendor: &'static str,
        dispatch_queue: Vec<DispatchBehavior>,
        retrieval_queue: Vec<RetrievalBehavior>,
    ) -> Self {
        Self {
            inner: Arc::new(ScriptedProviderInner {
                provider_modality,
                provider_vendor,
                dispatch_queue: Mutex::new(dispatch_queue),
                retrieval_queue: Mutex::new(retrieval_queue),
                retrieved_records: Mutex::new(Vec::new()),
            }),
        }
    }

    fn retrieved_vendor_values(&self) -> Vec<Option<String>> {
        self.inner
            .retrieved_records
            .lock()
            .unwrap()
            .iter()
            .map(|record| record.source_provider.clone())
            .collect()
    }
}

fn media_resource(kind: &str, content_type: &str, url: &str, duration_ms: Option<i64>) -> crate::domain::models::MediaResource {
    crate::domain::models::MediaResource {
        media_resource_id: None,
        kind: Some(kind.to_string()),
        source: Some("generated".to_string()),
        url: Some(url.to_string()),
        public_url: Some(url.to_string()),
        uri: Some(url.to_string()),
        media_type: Some(kind.to_string()),
        content_type: Some(content_type.to_string()),
        width: None,
        height: None,
        duration_ms,
        size_bytes: None,
        checksum_sha256: None,
        metadata: None,
    }
}

fn terminal_outcome(
    record: &GenerationRecord,
    result_type: &str,
    url: &str,
    duration_ms: Option<i64>,
) -> GenerationDispatchOutcome {
    let mut succeeded = record.clone();
    succeeded.status = GenerationStatus::Succeeded;
    succeeded.result_count = 1;
    let (kind, content_type) = if result_type == "video" {
        ("video", "video/mp4")
    } else {
        ("image", "image/png")
    };
    let result = GenerationResult {
        id: format!("{}:{result_type}-1", record.id),
        generation_id: record.id.clone(),
        result_type: result_type.to_string(),
        drive_space_id: None,
        drive_node_id: None,
        drive_uri: None,
        resource_snapshot: Some(media_resource(kind, content_type, url, duration_ms)),
        asset_id: None,
        preview_text: record.prompt_preview.clone(),
        created_at: "2026-09-12T00:02:00+00:00".to_string(),
    };
    GenerationDispatchOutcome {
        record: succeeded,
        results: vec![result],
        timeline_events: Vec::new(),
        usage: None,
    }
}

#[async_trait]
impl GenerationProvider for ScriptedProvider {
    fn modality(&self) -> GenerationModality {
        match self.inner.provider_modality {
            GenerationModality::Image => GenerationModality::Image,
            GenerationModality::Video => GenerationModality::Video,
            GenerationModality::Music => GenerationModality::Music,
            GenerationModality::Audio => GenerationModality::Audio,
            GenerationModality::Sfx => GenerationModality::Sfx,
            GenerationModality::Voice => GenerationModality::Voice,
        }
    }

    fn operation_types(&self) -> Vec<&str> {
        vec!["text_to_image", "image_edit", "text_to_video", "image_to_video"]
    }

    fn vendor(&self) -> &'static str {
        self.inner.provider_vendor
    }

    async fn dispatch(
        &self,
        record: &GenerationRecord,
        _command: &CreateGenerationCommandRequest,
        _context: &GenerationsRequestContext,
    ) -> Result<GenerationDispatchOutcome, GenerationsError> {
        let behavior = self
            .inner.dispatch_queue
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| {
                GenerationsError::Provider("no scripted dispatch behavior".to_string())
            })?;
        match behavior {
            DispatchBehavior::PendOnTask { vendor, task_id } => {
                let mut pending = record.clone();
                pending.status = GenerationStatus::Running;
                pending.source_provider = Some(vendor.to_string());
                pending.source_job_id = Some(task_id.to_string());
                Ok(GenerationDispatchOutcome::from_record(pending))
            }
            DispatchBehavior::CompleteWithImage { url } => {
                Ok(terminal_outcome(record, "image", url, None))
            }
            DispatchBehavior::Fail { message } => {
                Err(GenerationsError::Provider(message.to_string()))
            }
        }
    }

    async fn retrieve(
        &self,
        record: &GenerationRecord,
        _context: &GenerationsRequestContext,
    ) -> Result<Option<GenerationDispatchOutcome>, GenerationsError> {
        self.inner.retrieved_records.lock().unwrap().push(record.clone());
        let behavior = self.inner.retrieval_queue.lock().unwrap().pop();
        Ok(behavior.map(|behavior| match behavior {
            RetrievalBehavior::CompleteWithVideo { url } => {
                terminal_outcome(record, "video", url, Some(8000))
            }
            RetrievalBehavior::CompleteWithImage { url } => {
                terminal_outcome(record, "image", url, None)
            }
            RetrievalBehavior::StillRunning => {
                let mut pending = record.clone();
                pending.status = GenerationStatus::Running;
                GenerationDispatchOutcome::from_record(pending)
            }
        }))
    }
}

fn command(model: &str) -> CreateGenerationCommandRequest {
    CreateGenerationCommandRequest {
        tenant_id: "tenant-1".to_string(),
        organization_id: None,
        prompt: "a cinematic plateau".to_string(),
        model: Some(model.to_string()),
        input_asset_ids: None,
        parameters: None,
    }
}

fn context() -> GenerationsRequestContext {
    GenerationsRequestContext::from_parts(
        "tenant-1".to_string(),
        "user-1".to_string(),
        "trace-1".to_string(),
    )
}

fn service_state(
    providers: Vec<Box<dyn GenerationProvider>>,
) -> crate::service::GenerationsServiceState {
    crate::service::GenerationsServiceState::new(
        Arc::new(InMemoryGenerationRepository::default()),
        Arc::new(InMemoryResultRepository::default()),
        Arc::new(InMemoryTimelineRepository::default()),
        Arc::new(GenerationsConfig::default()),
        Arc::new(providers),
        Arc::new(NoopAssetPort),
        Arc::new(RecordingUsagePort::default()),
    )
}

// ---------------------------------------------------------------------------
// Image lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sync_image_dispatch_completes_without_refresh() {
    let openai = ScriptedProvider::new(
        GenerationModality::Image,
        "openai",
        vec![DispatchBehavior::CompleteWithImage {
            url: "https://cdn.example/openai-image.png",
        }],
        vec![],
    );
    let state = service_state(vec![Box::new(openai)]);

    let response = GenerationsService::create_generation(
        &state,
        &context(),
        GenerationModality::Image,
        "text_to_image",
        &command("openai/gpt-image-2"),
    )
    .await
    .expect("sync image generation succeeds");

    assert_eq!(response.generation.status, GenerationStatus::Succeeded);
    assert_eq!(
        response.generation.source_provider.as_deref(),
        Some("openai"),
        "sync vendors keep the dispatching vendor"
    );
    let results = state
        .result_repository()
        .list(ListResultsParams {
            generation_id: response.generation.id.clone(),
            cursor: None,
            page_size: Some(20),
        })
        .await
        .expect("results listed")
        .0;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].result_type, "image");
}

#[tokio::test]
async fn async_image_task_refresh_routes_by_persisted_vendor_and_reaches_terminal() {
    // Dispatch happens on the default provider (openai), but the command
    // resolved the google/nano-banana surface; the dispatch outcome persists
    // the resolved vendor exactly like the real adapter does.
    let openai = ScriptedProvider::new(
        GenerationModality::Image,
        "openai",
        vec![DispatchBehavior::PendOnTask {
            vendor: "nano-banana",
            task_id: "nano-task-1",
        }],
        vec![],
    );
    let google = ScriptedProvider::new(
        GenerationModality::Image,
        "nano-banana",
        vec![],
        vec![RetrievalBehavior::CompleteWithImage {
            url: "https://cdn.example/nano-banana-image.png",
        }],
    );
    let state = service_state(vec![Box::new(openai), Box::new(google.clone())]);

    let created = GenerationsService::create_generation(
        &state,
        &context(),
        GenerationModality::Image,
        "text_to_image",
        &command("google/gemini-2.5-flash-image"),
    )
    .await
    .expect("dispatch pends");
    assert_eq!(created.generation.status, GenerationStatus::Running);
    assert_eq!(
        created.generation.source_provider.as_deref(),
        Some("nano-banana"),
        "the resolved vendor must be persisted at dispatch time"
    );
    assert_eq!(created.generation.source_job_id.as_deref(), Some("nano-task-1"));

    let refreshed = GenerationsService::get_generation(&state, &context(), &created.generation.id)
        .await
        .expect("refresh reaches the vendor surface");
    assert_eq!(refreshed.status, GenerationStatus::Succeeded);
    assert_eq!(google.retrieved_vendor_values(), vec![Some("nano-banana".to_string())],
        "the refresh must poll the provider whose vendor matches the persisted record");
    let results = state
        .result_repository()
        .list(ListResultsParams {
            generation_id: refreshed.id.clone(),
            cursor: None,
            page_size: Some(20),
        })
        .await
        .expect("results listed")
        .0;
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].resource_snapshot.as_ref().unwrap().url.as_deref(),
        Some("https://cdn.example/nano-banana-image.png")
    );
}

// ---------------------------------------------------------------------------
// Video lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_video_task_refresh_routes_to_kling_surface_and_collects_media() {
    let openai = ScriptedProvider::new(
        GenerationModality::Video,
        "openai",
        vec![DispatchBehavior::PendOnTask {
            vendor: "kling",
            task_id: "kling-task-7",
        }],
        vec![],
    );
    let kling = ScriptedProvider::new(
        GenerationModality::Video,
        "kling",
        vec![],
        // pop() drains from the tail: the first refresh sees the task still
        // running, the second one collects the finished video.
        vec![
            RetrievalBehavior::CompleteWithVideo {
                url: "https://cdn.example/kling-final.mp4",
            },
            RetrievalBehavior::StillRunning,
        ],
    );
    let state = service_state(vec![Box::new(openai), Box::new(kling.clone())]);

    let created = GenerationsService::create_generation(
        &state,
        &context(),
        GenerationModality::Video,
        "text_to_video",
        &command("kling/kling-v2-master"),
    )
    .await
    .expect("dispatch pends");
    assert_eq!(created.generation.status, GenerationStatus::Running);
    assert_eq!(created.generation.source_provider.as_deref(), Some("kling"));
    assert_eq!(created.generation.source_job_id.as_deref(), Some("kling-task-7"));

    // A still-running vendor keeps the record pending across refreshes.
    let pending = GenerationsService::get_generation(&state, &context(), &created.generation.id)
        .await
        .expect("first refresh keeps the task pending");
    assert_eq!(pending.status, GenerationStatus::Running);

    let refreshed = GenerationsService::get_generation(&state, &context(), &created.generation.id)
        .await
        .expect("second refresh collects the finished task");
    assert_eq!(refreshed.status, GenerationStatus::Succeeded);
    assert_eq!(
        kling.retrieved_vendor_values(),
        vec![Some("kling".to_string()), Some("kling".to_string())],
        "every refresh must route to the kling surface by the persisted vendor"
    );
    let results = state
        .result_repository()
        .list(ListResultsParams {
            generation_id: refreshed.id.clone(),
            cursor: None,
            page_size: Some(20),
        })
        .await
        .expect("results listed")
        .0;
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].resource_snapshot.as_ref().unwrap().url.as_deref(),
        Some("https://cdn.example/kling-final.mp4")
    );
    assert_eq!(
        results[0].resource_snapshot.as_ref().unwrap().duration_ms,
        Some(8000)
    );
}

#[tokio::test]
async fn failed_video_dispatch_persists_failed_record() {
    let volcengine = ScriptedProvider::new(
        GenerationModality::Video,
        "volcengine",
        vec![DispatchBehavior::Fail {
            message: "volcengine task rejected the prompt",
        }],
        vec![],
    );
    let state = service_state(vec![Box::new(volcengine)]);

    let error = GenerationsService::create_generation(
        &state,
        &context(),
        GenerationModality::Video,
        "text_to_video",
        &command("volcengine/doubao-seedance-1-0-pro"),
    )
    .await
    .expect_err("provider failure surfaces");
    assert!(error.to_string().contains("volcengine task rejected"));

    let records = state
        .repository()
        .list(ListGenerationsParams {
            tenant_id: "tenant-1".to_string(),
            cursor: None,
            page_size: Some(10),
            status: None,
            modality: Some("video".to_string()),
            operation_type: None,
            q: None,
        })
        .await
        .expect("records listed")
        .0;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, GenerationStatus::Failed);
}
