//! MCP port trait plus production and in-memory implementations.

use async_trait::async_trait;
use sdkwork_intelligence_generations_service::domain::models::{
    CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationResult,
};
use sdkwork_intelligence_generations_service::error::GenerationsError;
use sdkwork_intelligence_generations_service::ports::{ListResultsParams, ListTimelineParams};
use sdkwork_intelligence_generations_service::{GenerationsRequestContext, GenerationsService, GenerationsServiceState};

/// Port the MCP tools use to reach the generations service.
#[async_trait]
pub trait GenerationsMcpPort: Send + Sync {
    /// Create a generation command for a modality and operation type.
    async fn create_generation(
        &self,
        modality: GenerationModality,
        operation_type: &str,
        command: &CreateGenerationCommandRequest,
    ) -> Result<GenerationRecord, GenerationsError>;

    /// Retrieve a generation record (refreshing async tasks on read).
    async fn get_generation(&self, generation_id: &str) -> Result<GenerationRecord, GenerationsError>;

    /// List the persisted results of a generation.
    async fn list_results(
        &self,
        generation_id: &str,
        params: ListResultsParams,
    ) -> Result<(Vec<GenerationResult>, sdkwork_intelligence_generations_service::domain::models::PageInfo), GenerationsError>;

    /// List the timeline events of a generation.
    async fn list_timeline(
        &self,
        generation_id: &str,
        params: ListTimelineParams,
    ) -> Result<(Vec<sdkwork_intelligence_generations_service::domain::models::GenerationTimelineEvent>, sdkwork_intelligence_generations_service::domain::models::PageInfo), GenerationsError>;
}

/// Production port backed by the generations service state.
pub struct StateGenerationsMcpPort {
    state: GenerationsServiceState,
    tenant_id: String,
    user_id: String,
}

impl StateGenerationsMcpPort {
    /// Build the port from a service state.
    pub fn new(state: GenerationsServiceState) -> Self {
        Self {
            state,
            tenant_id: std::env::var("GENERATIONS_MCP_TENANT_ID").unwrap_or_else(|_| "0".to_string()),
            user_id: std::env::var("GENERATIONS_MCP_USER_ID").unwrap_or_else(|_| "mcp-agent".to_string()),
        }
    }

    fn context(&self) -> GenerationsRequestContext {
        GenerationsRequestContext {
            http: sdkwork_intelligence_generations_service::GenerationsHttpRequestContext {
                tenant_id: self.tenant_id.clone(),
                user_id: self.user_id.clone(),
                trace_id: uuid::Uuid::new_v4().to_string(),
            },
        }
    }
}

#[async_trait]
impl GenerationsMcpPort for StateGenerationsMcpPort {
    async fn create_generation(
        &self,
        modality: GenerationModality,
        operation_type: &str,
        command: &CreateGenerationCommandRequest,
    ) -> Result<GenerationRecord, GenerationsError> {
        let context = self.context();
        let response = GenerationsService::create_generation(
            &self.state,
            &context,
            modality,
            operation_type,
            command,
        )
        .await?;
        Ok(response.generation)
    }

    async fn get_generation(&self, generation_id: &str) -> Result<GenerationRecord, GenerationsError> {
        let context = self.context();
        GenerationsService::get_generation(&self.state, &context, generation_id).await
    }

    async fn list_results(
        &self,
        generation_id: &str,
        params: ListResultsParams,
    ) -> Result<(Vec<GenerationResult>, sdkwork_intelligence_generations_service::domain::models::PageInfo), GenerationsError> {
        let context = self.context();
        GenerationsService::list_results(&self.state, &context, generation_id, params).await
    }

    async fn list_timeline(
        &self,
        generation_id: &str,
        params: ListTimelineParams,
    ) -> Result<(Vec<sdkwork_intelligence_generations_service::domain::models::GenerationTimelineEvent>, sdkwork_intelligence_generations_service::domain::models::PageInfo), GenerationsError> {
        GenerationsService::list_timeline(&self.state, generation_id, params).await
    }
}

/// In-memory port for tests and offline tooling.
#[derive(Default)]
pub struct InMemoryGenerationsMcpPort {
    records: tokio::sync::Mutex<Vec<GenerationRecord>>,
}

impl InMemoryGenerationsMcpPort {
    /// Create an empty in-memory port.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a record so retrieve flows can be exercised in tests.
    pub async fn seed(&self, record: GenerationRecord) {
        self.records.lock().await.push(record);
    }
}

#[async_trait]
impl GenerationsMcpPort for InMemoryGenerationsMcpPort {
    async fn create_generation(
        &self,
        modality: GenerationModality,
        operation_type: &str,
        command: &CreateGenerationCommandRequest,
    ) -> Result<GenerationRecord, GenerationsError> {
        let now = chrono::Utc::now().to_rfc3339();
        let record = GenerationRecord {
            id: uuid::Uuid::new_v4().to_string(),
            tenant_id: command.tenant_id.clone(),
            organization_id: command.organization_id.clone(),
            user_id: "mcp-agent".to_string(),
            modality,
            operation_type: operation_type.to_string(),
            source_provider: Some("in-memory".to_string()),
            source_job_id: None,
            prompt_preview: Some(command.prompt.chars().take(200).collect()),
            status: sdkwork_intelligence_generations_service::domain::models::GenerationStatus::Succeeded,
            favorite: false,
            result_count: 0,
            created_at: now.clone(),
            updated_at: now,
        };
        self.records.lock().await.push(record.clone());
        Ok(record)
    }

    async fn get_generation(&self, generation_id: &str) -> Result<GenerationRecord, GenerationsError> {
        self.records
            .lock()
            .await
            .iter()
            .find(|record| record.id == generation_id)
            .cloned()
            .ok_or_else(|| GenerationsError::NotFound(generation_id.to_string()))
    }

    async fn list_results(
        &self,
        _generation_id: &str,
        _params: ListResultsParams,
    ) -> Result<(Vec<GenerationResult>, sdkwork_intelligence_generations_service::domain::models::PageInfo), GenerationsError> {
        Ok((
            Vec::new(),
            sdkwork_intelligence_generations_service::domain::models::PageInfo::cursor(None, false),
        ))
    }

    async fn list_timeline(
        &self,
        _generation_id: &str,
        _params: ListTimelineParams,
    ) -> Result<(Vec<sdkwork_intelligence_generations_service::domain::models::GenerationTimelineEvent>, sdkwork_intelligence_generations_service::domain::models::PageInfo), GenerationsError> {
        Ok((
            Vec::new(),
            sdkwork_intelligence_generations_service::domain::models::PageInfo::cursor(None, false),
        ))
    }
}
