//! Gateway bootstrap for sdkwork-generations.

use std::sync::Arc;

use axum::Router;
use sdkwork_database_sqlx::DatabasePool;
pub use sdkwork_web_bootstrap::ApiAssemblyContribution;
use sdkwork_web_bootstrap::{PgPoolReadinessCheck, ReadinessCheck};
use sdkwork_web_core::HttpRouteManifest;

pub type ApiAssembly = ApiAssemblyContribution;

async fn assemble_business_routes(pool: sqlx::PgPool) -> Router {
    let generation_repo = Arc::new(repository::PostgresGenerationRepository::new(pool.clone()));
    let result_repo = Arc::new(repository::PostgresGenerationResultRepository::new(pool.clone()));
    let timeline_repo = Arc::new(repository::PostgresTimelineRepository::new(pool.clone()));
    let config = Arc::new(sdkwork_intelligence_generations_service::GenerationsConfig::from_env());
    let providers = sdkwork_generations_provider_adapter::build_providers_from_env()
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, "generations providers disabled; creation endpoints will fail");
            Arc::new(Vec::new())
        });
    let asset_port = Arc::new(repository::NoopAssetPort);
    let usage_port = Arc::new(repository::PostgresGenerationUsageRecorder::new(pool.clone()));

    let state = sdkwork_intelligence_generations_service::GenerationsServiceState::new(
        generation_repo,
        result_repo,
        timeline_repo,
        config,
        providers,
        asset_port,
        usage_port,
    );

    let app_router = sdkwork_routes_generations_app_api::gateway_mount(state.clone()).await;
    let backend_router = sdkwork_routes_generations_backend_api::gateway_mount(state).await;
    app_router.merge(backend_router)
}

fn build_api_contribution(
    router: Router,
    readiness_check: Arc<dyn ReadinessCheck>,
) -> Result<ApiAssembly, String> {
    ApiAssemblyContribution::from_openapi_documents(
        "sdkwork-generations",
        "SDKWork Generations App API",
        router,
        build_route_manifest(),
        openapi_documents()?,
        vec![sdkwork_routes_drive_app_api::drive_app_context_injector()],
        readiness_check,
    )
}

fn openapi_documents() -> Result<Vec<serde_json::Value>, String> {
    [
        (
            "sdkwork-generations-app-api",
            include_str!("../../../sdks/sdkwork-generations-app-sdk/openapi/sdkwork-generations-app-api.openapi.json"),
        ),
        (
            "sdkwork-generations-backend-api",
            include_str!("../../../sdks/sdkwork-generations-backend-sdk/openapi/sdkwork-generations-backend-api.openapi.json"),
        ),
    ]
    .into_iter()
    .map(|(owner, source)| {
        serde_json::from_str(source).map_err(|error| format!("invalid {owner} OpenAPI: {error}"))
    })
    .collect()
}

/// Assemble the generations application router from environment variables.
pub async fn assemble_api_router() -> Result<ApiAssembly, String> {
    let host = bootstrap_generations_database_from_env().await?;
    let pool = host
        .as_postgres()
        .ok_or_else(|| "Generations assembly requires PostgreSQL".to_string())?
        .clone();
    build_api_contribution(
        assemble_business_routes(pool.clone()).await,
        Arc::new(PgPoolReadinessCheck::new(pool)),
    )
}

/// Assemble the generations application router against a caller-provided database pool
/// so the platform cloud gateway can share its process-wide PostgreSQL pool.
pub async fn assemble_api_router_with_pool(pool: DatabasePool) -> Result<ApiAssembly, String> {
    let host = bootstrap_generations_database_with_pool(pool).await?;
    let pg_pool = host
        .as_postgres()
        .ok_or_else(|| "Generations assembly requires PostgreSQL".to_string())?
        .clone();
    build_api_contribution(
        assemble_business_routes(pg_pool.clone()).await,
        Arc::new(PgPoolReadinessCheck::new(pg_pool)),
    )
}

async fn bootstrap_generations_database_from_env() -> Result<DatabasePool, String> {
    sdkwork_generations_database_host::bootstrap_generations_database_from_env()
        .await
        .map_err(|e| e.to_string())
}

async fn bootstrap_generations_database_with_pool(pool: DatabasePool) -> Result<DatabasePool, String> {
    Ok(pool)
}

/// Runs the generations-owned database lifecycle without constructing HTTP routes.
pub async fn bootstrap_database_from_env() -> Result<(), String> {
    bootstrap_generations_database_from_env().await.map(|_| ())
}

/// Builds the Generations App API contribution for gateway composition.
pub async fn assemble_app_api_contribution() -> Result<ApiAssemblyContribution, String> {
    assemble_api_router().await
}

fn build_route_manifest() -> HttpRouteManifest {
    sdkwork_routes_generations_http_shared::combined_route_manifest()
}

/// Repository module.
mod repository {
    use async_trait::async_trait;
    use serde_json::Value;

    use sdkwork_intelligence_generations_service::domain::models::{
        GenerationModality, GenerationRecord, GenerationResult, GenerationStatus,
        GenerationTimelineEvent, SaveGenerationResultToAssetsRequest,
    };
    use sdkwork_intelligence_generations_service::error::GenerationsError;
    use sdkwork_intelligence_generations_service::context::GenerationsRequestContext;
    use sdkwork_intelligence_generations_service::ports::{
        AssetPort, CreateGenerationParams, GenerationRepository, GenerationResultRepository,
        GenerationUsageFact, GenerationUsagePort, ListGenerationsParams, ListResultsParams,
        ListTimelineParams, TimelineRepository, UpdateGenerationProviderStateParams,
    };

    // -----------------------------------------------------------------------
    // PostgreSQL generation record repository
    // -----------------------------------------------------------------------

    pub struct PostgresGenerationRepository {
        pool: sqlx::PgPool,
    }

    impl PostgresGenerationRepository {
        pub fn new(pool: sqlx::PgPool) -> Self {
            Self { pool }
        }
    }

    #[async_trait]
    impl GenerationRepository for PostgresGenerationRepository {
        async fn create(
            &self,
            params: CreateGenerationParams,
        ) -> Result<GenerationRecord, GenerationsError> {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            let input_refs = serde_json::to_value(&params.input_asset_ids).map_err(|e| {
                GenerationsError::Internal(format!("failed to serialize input refs: {e}"))
            })?;

            sqlx::query(
                r#"
                INSERT INTO generation_record (
                    id, tenant_id, organization_id, user_id, modality, status, operation_type,
                    source_provider, source_job_id, prompt_preview, parameter_snapshot,
                    input_refs_json, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, 'queued', $6, $7, $8, $9, $10, $11,
                        CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                "#,
            )
            .bind(&id)
            .bind(&params.tenant_id)
            .bind(&params.organization_id)
            .bind(&params.user_id)
            .bind(&params.modality)
            .bind(&params.operation_type)
            .bind(&params.source_provider)
            .bind(&params.source_job_id)
            .bind(&params.prompt_preview)
            .bind(&params.metadata)
            .bind(&input_refs)
            .execute(&self.pool)
            .await?;

            Ok(GenerationRecord {
                id,
                tenant_id: params.tenant_id,
                organization_id: params.organization_id,
                user_id: params.user_id,
                modality: GenerationModality::parse(&params.modality).unwrap_or(GenerationModality::Image),
                status: GenerationStatus::Queued,
                operation_type: params.operation_type,
                source_provider: params.source_provider,
                source_job_id: params.source_job_id,
                prompt_preview: params.prompt_preview,
                favorite: false,
                result_count: 0,
                created_at: now.clone(),
                updated_at: now,
            })
        }

        async fn get(&self, id: &str) -> Result<Option<GenerationRecord>, GenerationsError> {
            let row = sqlx::query_as::<_, GenerationRecordRow>(
                "SELECT id, tenant_id, organization_id, user_id, modality, status, operation_type, source_provider, source_job_id, prompt_preview, favorite, result_count, created_at, updated_at FROM generation_record WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

            Ok(row.map(|r| r.into_record()))
        }

        async fn list(
            &self,
            params: ListGenerationsParams,
        ) -> Result<(Vec<GenerationRecord>, Option<String>, bool), GenerationsError> {
            let page_size = params.page_size.unwrap_or(20).min(100);
            let rows = sqlx::query_as::<_, GenerationRecordRow>(
                r#"
                SELECT id, tenant_id, organization_id, user_id, modality, status, operation_type, source_provider, source_job_id, prompt_preview, favorite, result_count, created_at, updated_at
                FROM generation_record
                WHERE tenant_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
            .bind(&params.tenant_id)
            .bind(page_size + 1)
            .fetch_all(&self.pool)
            .await?;

            let has_more = rows.len() > page_size as usize;
            let items: Vec<GenerationRecord> = rows
                .into_iter()
                .take(page_size as usize)
                .map(|r| r.into_record())
                .collect();

            let next_cursor = if has_more {
                items.last().map(|r| r.id.clone())
            } else {
                None
            };

            Ok((items, next_cursor, has_more))
        }

        async fn cancel(
            &self,
            id: &str,
            _reason: Option<&str>,
        ) -> Result<Option<GenerationRecord>, GenerationsError> {
            let rows_affected = sqlx::query(
                "UPDATE generation_record SET status = 'canceled', updated_at = $2 WHERE id = $1 AND status NOT IN ('succeeded', 'failed', 'canceled')"
            )
            .bind(id)
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?
            .rows_affected();

            if rows_affected == 0 {
                return self.get(id).await;
            }

            self.get(id).await
        }

        async fn retry(
            &self,
            id: &str,
            _reason: Option<&str>,
        ) -> Result<Option<GenerationRecord>, GenerationsError> {
            let rows_affected = sqlx::query(
                "UPDATE generation_record SET status = 'queued', updated_at = $2 WHERE id = $1 AND status IN ('failed', 'canceled')"
            )
            .bind(id)
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?
            .rows_affected();

            if rows_affected == 0 {
                return Ok(None);
            }

            self.get(id).await
        }

        async fn set_favorite(
            &self,
            id: &str,
            favorite: bool,
        ) -> Result<Option<GenerationRecord>, GenerationsError> {
            let rows_affected = sqlx::query(
                "UPDATE generation_record SET favorite = $2, updated_at = $3 WHERE id = $1"
            )
            .bind(id)
            .bind(favorite)
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?
            .rows_affected();

            if rows_affected == 0 {
                return Ok(None);
            }

            self.get(id).await
        }

        async fn update_provider_state(
            &self,
            params: UpdateGenerationProviderStateParams,
        ) -> Result<Option<GenerationRecord>, GenerationsError> {
            let result = sqlx::query(
                r#"
                UPDATE generation_record SET
                    status = COALESCE($2, status),
                    source_job_id = COALESCE($3, source_job_id),
                    result_count = COALESCE($4, result_count),
                    error_code = CASE WHEN $6 THEN NULL ELSE COALESCE($5, error_code) END,
                    error_message = CASE WHEN $6 THEN NULL ELSE COALESCE($7, error_message) END,
                    completed_at = CASE
                        WHEN $2 IN ('succeeded', 'failed', 'canceled') THEN CURRENT_TIMESTAMP
                        ELSE completed_at
                    END,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $1
                "#,
            )
            .bind(&params.id)
            .bind(&params.status)
            .bind(&params.source_job_id)
            .bind(params.result_count)
            .bind(&params.error_code)
            .bind(params.clear_error)
            .bind(&params.error_message)
            .execute(&self.pool)
            .await?;

            if result.rows_affected() == 0 {
                return Ok(None);
            }
            self.get(&params.id).await
        }
    }

    #[derive(sqlx::FromRow)]
    struct GenerationRecordRow {
        id: String,
        tenant_id: String,
        organization_id: Option<String>,
        user_id: String,
        modality: String,
        status: String,
        operation_type: String,
        source_provider: Option<String>,
        source_job_id: Option<String>,
        prompt_preview: Option<String>,
        favorite: bool,
        result_count: i32,
        created_at: String,
        updated_at: String,
    }

    impl GenerationRecordRow {
        fn into_record(self) -> GenerationRecord {
            GenerationRecord {
                id: self.id,
                tenant_id: self.tenant_id,
                organization_id: self.organization_id,
                user_id: self.user_id,
                modality: GenerationModality::parse(&self.modality).unwrap_or(GenerationModality::Image),
                status: GenerationStatus::parse(&self.status).unwrap_or(GenerationStatus::Queued),
                operation_type: self.operation_type,
                source_provider: self.source_provider,
                source_job_id: self.source_job_id,
                prompt_preview: self.prompt_preview,
                favorite: self.favorite,
                result_count: self.result_count,
                created_at: self.created_at,
                updated_at: self.updated_at,
            }
        }
    }

    // -----------------------------------------------------------------------
    // PostgreSQL generation result repository
    // -----------------------------------------------------------------------

    pub struct PostgresGenerationResultRepository {
        pool: sqlx::PgPool,
    }

    impl PostgresGenerationResultRepository {
        pub fn new(pool: sqlx::PgPool) -> Self {
            Self { pool }
        }
    }

    #[async_trait]
    impl GenerationResultRepository for PostgresGenerationResultRepository {
        async fn create(&self, result: &GenerationResult) -> Result<GenerationResult, GenerationsError> {
            let snapshot_json = match &result.resource_snapshot {
                Some(resource) => Some(serde_json::to_value(resource).map_err(|e| {
                    GenerationsError::Internal(format!("failed to serialize resource_snapshot: {e}"))
                })?),
                None => None,
            };
            sqlx::query(
                r#"
                INSERT INTO generation_result (
                    id, generation_id, tenant_id, result_type, drive_space_id, drive_node_id,
                    drive_uri, resource_snapshot, asset_id, preview_text, created_at
                )
                VALUES (
                    COALESCE($1, 'gen-result-' || gen_random_uuid()::text),
                    $2,
                    (SELECT tenant_id FROM generation_record WHERE id = $2),
                    $3, $4, $5, $6, $7, $8, $9, CURRENT_TIMESTAMP
                )
                ON CONFLICT (id) DO NOTHING
                "#,
            )
            .bind(&result.id)
            .bind(&result.generation_id)
            .bind(&result.result_type)
            .bind(&result.drive_space_id)
            .bind(&result.drive_node_id)
            .bind(&result.drive_uri)
            .bind(&snapshot_json)
            .bind(&result.asset_id)
            .bind(&result.preview_text)
            .execute(&self.pool)
            .await?;

            Ok(result.clone())
        }

        async fn get(
            &self,
            generation_id: &str,
            result_id: &str,
        ) -> Result<Option<GenerationResult>, GenerationsError> {
            let row = sqlx::query_as::<_, GenerationResultRow>(
                "SELECT id, generation_id, result_type, drive_space_id, drive_node_id, drive_uri, resource_snapshot, asset_id, preview_text, created_at FROM generation_result WHERE id = $1 AND generation_id = $2",
            )
            .bind(result_id)
            .bind(generation_id)
            .fetch_optional(&self.pool)
            .await?;

            Ok(row.map(|r| r.into_result()))
        }

        async fn list(
            &self,
            params: ListResultsParams,
        ) -> Result<(Vec<GenerationResult>, Option<String>, bool), GenerationsError> {
            let page_size = params.page_size.unwrap_or(20).min(100);
            let rows = sqlx::query_as::<_, GenerationResultRow>(
                r#"
                SELECT id, generation_id, result_type, drive_space_id, drive_node_id, drive_uri, resource_snapshot, asset_id, preview_text, created_at
                FROM generation_result
                WHERE generation_id = $1
                ORDER BY created_at ASC
                LIMIT $2
                "#,
            )
            .bind(&params.generation_id)
            .bind(page_size + 1)
            .fetch_all(&self.pool)
            .await?;

            let has_more = rows.len() > page_size as usize;
            let items: Vec<GenerationResult> = rows
                .into_iter()
                .take(page_size as usize)
                .map(|r| r.into_result())
                .collect();

            let next_cursor = if has_more {
                items.last().map(|r| r.id.clone())
            } else {
                None
            };

            Ok((items, next_cursor, has_more))
        }

        async fn update(
            &self,
            result: &GenerationResult,
        ) -> Result<GenerationResult, GenerationsError> {
            let snapshot_json = match &result.resource_snapshot {
                Some(resource) => Some(serde_json::to_value(resource).map_err(|e| {
                    GenerationsError::Internal(format!("failed to serialize resource_snapshot: {e}"))
                })?),
                None => None,
            };
            sqlx::query(
                r#"
                UPDATE generation_result
                SET result_type = $2, drive_space_id = $3, drive_node_id = $4, drive_uri = $5,
                    resource_snapshot = $6, asset_id = $7, preview_text = $8
                WHERE id = $1
                "#,
            )
            .bind(&result.id)
            .bind(&result.result_type)
            .bind(&result.drive_space_id)
            .bind(&result.drive_node_id)
            .bind(&result.drive_uri)
            .bind(&snapshot_json)
            .bind(&result.asset_id)
            .bind(&result.preview_text)
            .execute(&self.pool)
            .await?;

            Ok(result.clone())
        }
    }

    #[derive(sqlx::FromRow)]
    struct GenerationResultRow {
        id: String,
        generation_id: String,
        result_type: String,
        drive_space_id: Option<String>,
        drive_node_id: Option<String>,
        drive_uri: Option<String>,
        resource_snapshot: Option<serde_json::Value>,
        asset_id: Option<String>,
        preview_text: Option<String>,
        created_at: String,
    }

    impl GenerationResultRow {
        fn into_result(self) -> GenerationResult {
            GenerationResult {
                id: self.id,
                generation_id: self.generation_id,
                result_type: self.result_type,
                drive_space_id: self.drive_space_id,
                drive_node_id: self.drive_node_id,
                drive_uri: self.drive_uri,
                resource_snapshot: serde_json::from_value(
                    self.resource_snapshot.unwrap_or(serde_json::Value::Null),
                )
                .ok()
                .flatten(),
                asset_id: self.asset_id,
                preview_text: self.preview_text,
                created_at: self.created_at,
            }
        }
    }

    // -----------------------------------------------------------------------
    // PostgreSQL timeline repository
    // -----------------------------------------------------------------------

    pub struct PostgresTimelineRepository {
        pool: sqlx::PgPool,
    }

    impl PostgresTimelineRepository {
        pub fn new(pool: sqlx::PgPool) -> Self {
            Self { pool }
        }
    }

    #[async_trait]
    impl TimelineRepository for PostgresTimelineRepository {
        async fn append(
            &self,
            event: &GenerationTimelineEvent,
        ) -> Result<GenerationTimelineEvent, GenerationsError> {
            sqlx::query(
                r#"
                INSERT INTO generation_timeline_event (id, generation_id, tenant_id, event_type, message, payload, created_at)
                VALUES (
                    COALESCE($1, 'gen-timeline-' || gen_random_uuid()::text),
                    $2,
                    (SELECT tenant_id FROM generation_record WHERE id = $2),
                    $3, $4, $5, CURRENT_TIMESTAMP
                )
                ON CONFLICT (id) DO NOTHING
                "#,
            )
            .bind(&event.id)
            .bind(&event.generation_id)
            .bind(&event.event_type)
            .bind(&event.message)
            .bind(&event.payload)
            .execute(&self.pool)
            .await?;

            Ok(event.clone())
        }

        async fn list(
            &self,
            params: ListTimelineParams,
        ) -> Result<(Vec<GenerationTimelineEvent>, Option<String>, bool), GenerationsError> {
            let page_size = params.page_size.unwrap_or(50).min(200);
            let rows = sqlx::query_as::<_, TimelineEventRow>(
                r#"
                SELECT id, generation_id, event_type, message, payload, created_at
                FROM generation_timeline_event
                WHERE generation_id = $1
                ORDER BY created_at ASC
                LIMIT $2
                "#,
            )
            .bind(&params.generation_id)
            .bind(page_size + 1)
            .fetch_all(&self.pool)
            .await?;

            let has_more = rows.len() > page_size as usize;
            let items: Vec<GenerationTimelineEvent> = rows
                .into_iter()
                .take(page_size as usize)
                .map(|r| r.into_event())
                .collect();

            let next_cursor = if has_more {
                items.last().map(|r| r.id.clone())
            } else {
                None
            };

            Ok((items, next_cursor, has_more))
        }
    }

    #[derive(sqlx::FromRow)]
    struct TimelineEventRow {
        id: String,
        generation_id: String,
        event_type: String,
        message: Option<String>,
        payload: Option<serde_json::Value>,
        created_at: String,
    }

    impl TimelineEventRow {
        fn into_event(self) -> GenerationTimelineEvent {
            GenerationTimelineEvent {
                id: self.id,
                generation_id: self.generation_id,
                event_type: self.event_type,
                message: self.message,
                payload: self.payload,
                created_at: self.created_at,
            }
        }
    }

    // -----------------------------------------------------------------------
    // PostgreSQL usage recorder (billing facts)
    // -----------------------------------------------------------------------

    /// Persists metering facts for billing: a `generation.usage.recorded`
    /// timeline event plus a `generation.usage.recorded` outbox event that
    /// downstream billing consumers drain.
    pub struct PostgresGenerationUsageRecorder {
        pool: sqlx::PgPool,
    }

    impl PostgresGenerationUsageRecorder {
        pub fn new(pool: sqlx::PgPool) -> Self {
            Self { pool }
        }
    }

    #[async_trait]
    impl GenerationUsagePort for PostgresGenerationUsageRecorder {
        async fn record_usage(&self, fact: &GenerationUsageFact) -> Result<(), GenerationsError> {
            let usage = &fact.usage;
            let payload = serde_json::json!({
                "generationId": fact.generation_id,
                "tenantId": fact.tenant_id,
                "userId": fact.user_id,
                "modality": fact.modality,
                "operationType": fact.operation_type,
                "source": fact.source.as_str(),
                "vendor": usage.vendor,
                "model": usage.model,
                "imageCount": usage.image_count,
                "videoSeconds": usage.video_seconds,
                "audioSeconds": usage.audio_seconds,
                "inputTokens": usage.input_tokens,
                "outputTokens": usage.output_tokens,
                "raw": usage.raw,
            });
            let fact_id = uuid::Uuid::new_v4().to_string();

            sqlx::query(
                r#"
                INSERT INTO generation_timeline_event (id, generation_id, tenant_id, event_type, message, payload)
                VALUES ($1, $2, $3, 'generation.usage.recorded', $4, $5)
                "#,
            )
            .bind(format!("{id}:usage", id = fact.generation_id))
            .bind(&fact.generation_id)
            .bind(&fact.tenant_id)
            .bind(format!(
                "usage recorded: vendor={vendor}, images={images}, videoSeconds={video}, audioSeconds={audio}",
                vendor = usage.vendor,
                images = usage.image_count,
                video = usage.video_seconds,
                audio = usage.audio_seconds
            ))
            .bind(&payload)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO generation_outbox_event (id, tenant_id, aggregate_type, aggregate_id, event_type, payload)
                VALUES ($1, $2, 'generation', $3, 'generation.usage.recorded', $4)
                "#,
            )
            .bind(fact_id)
            .bind(&fact.tenant_id)
            .bind(&fact.generation_id)
            .bind(&payload)
            .execute(&self.pool)
            .await?;

            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Noop asset port (placeholder for real implementation)
    // -----------------------------------------------------------------------

    pub struct NoopAssetPort;

    #[async_trait]
    impl AssetPort for NoopAssetPort {
        async fn save_generation_result(
            &self,
            _generation_id: &str,
            result_id: &str,
            _request: &SaveGenerationResultToAssetsRequest,
            _context: &GenerationsRequestContext,
        ) -> Result<GenerationResult, GenerationsError> {
            Err(GenerationsError::Asset(
                format!("asset port not configured; cannot save result {result_id}")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_api_manifest_matches_authored_openapi_inventory() {
        let manifest = build_route_manifest();
        let documents = openapi_documents().expect("authored OpenAPI documents parse");
        let manifest_inventory = manifest
            .routes()
            .iter()
            .map(|route| {
                (
                    method_name(route.method),
                    route.path,
                    route.operation_id,
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        let openapi_inventory = documents
            .iter()
            .flat_map(|document| {
                document["paths"]
                    .as_object()
                    .into_iter()
                    .flat_map(|paths| paths.iter())
                    .flat_map(|(path, item)| {
                        item.as_object().into_iter().flat_map(move |operations| {
                            operations.iter().filter_map(move |(method, operation)| {
                                operation["operationId"].as_str().map(|operation_id| {
                                    (method.as_str(), path.as_str(), operation_id)
                                })
                            })
                        })
                    })
            })
            .collect::<std::collections::BTreeSet<_>>();
        let missing = openapi_inventory
            .difference(&manifest_inventory)
            .cloned()
            .collect::<Vec<_>>();
        let extra = manifest_inventory
            .difference(&openapi_inventory)
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "generations API inventory drift; missing={missing:?}; extra={extra:?}"
        );
    }

    fn method_name(method: sdkwork_web_core::HttpMethod) -> &'static str {
        match method {
            sdkwork_web_core::HttpMethod::Delete => "delete",
            sdkwork_web_core::HttpMethod::Get => "get",
            sdkwork_web_core::HttpMethod::Patch => "patch",
            sdkwork_web_core::HttpMethod::Post => "post",
            sdkwork_web_core::HttpMethod::Put => "put",
        }
    }
}
