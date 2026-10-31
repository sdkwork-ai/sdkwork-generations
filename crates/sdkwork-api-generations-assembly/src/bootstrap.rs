//! Gateway bootstrap for sdkwork-generations.

use std::sync::Arc;

use axum::Router;
use sdkwork_database_sqlx::DatabasePool;
pub use sdkwork_web_bootstrap::ApiAssemblyContribution;
use sdkwork_web_bootstrap::{PgPoolReadinessCheck, ReadinessCheck, WebModule};
use sdkwork_web_core::HttpRouteManifest;

pub use sdkwork_intelligence_generations_service::domain::pricing::{
    GenerationPricingError, GenerationPricingEstimate, GenerationPricingEstimateLine,
    GenerationPricingEstimateRequest, GenerationPricingPort, GenerationPricingScope,
};

pub type ApiAssembly = ApiAssemblyContribution;

async fn assemble_business_routes(
    pool: sqlx::PgPool,
    pricing_port: Option<Arc<dyn GenerationPricingPort>>,
) -> Router {
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
    )
    .with_pricing_port(pricing_port.unwrap_or_else(|| {
        Arc::new(sdkwork_intelligence_generations_service::UnconfiguredGenerationPricingPort)
    }));

    let app_router = sdkwork_routes_generations_app_api::build_router().with_state(state.clone());
    let backend_router = sdkwork_routes_generations_backend_api::build_router().with_state(state);
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
        vec![sdkwork_routes_generations_http_shared::generations_request_context_injector()],
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
        assemble_business_routes(pool.clone(), None).await,
        Arc::new(PgPoolReadinessCheck::new(pool)),
    )
}

/// Assemble the generations application router against a caller-provided database pool
/// so the platform cloud gateway can share its process-wide PostgreSQL pool.
pub async fn assemble_api_router_with_pool(pool: DatabasePool) -> Result<ApiAssembly, String> {
    assemble_api_router_with_pool_and_pricing(pool, None).await
}

/// Assemble with an explicit pricing port so the composition host that owns
/// the credit-pricing engine (sdkwork-cloudrouter) can serve
/// `POST /app/v3/api/generations/pricing/estimate` with real rates. `None`
/// keeps the unconfigured port and the endpoint answers 501.
pub async fn assemble_api_router_with_pool_and_pricing(
    pool: DatabasePool,
    pricing_port: Option<Arc<dyn GenerationPricingPort>>,
) -> Result<ApiAssembly, String> {
    let host = bootstrap_generations_database_with_pool(pool).await?;
    let pg_pool = host
        .as_postgres()
        .ok_or_else(|| "Generations assembly requires PostgreSQL".to_string())?
        .clone();
    build_api_contribution(
        assemble_business_routes(pg_pool.clone(), pricing_port).await,
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

/// Generations App API route manifest for host gateway composition.
///
/// Host gateways that merge the Generations app surface contribution compose
/// this manifest into their own surface route inventory so the Web Framework
/// honors the Generations routes' declared authentication and permissions
/// (API_ASSEMBLY_SPEC §3).
pub fn app_api_route_manifest() -> HttpRouteManifest {
    build_route_manifest()
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
                    id, tenant_id, organization_id, user_id, conversation_id, modality, status,
                    operation_type, source_provider, source_job_id, prompt_preview,
                    parameter_snapshot, input_refs_json, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, 'queued', $7, $8, $9, $10, $11, $12,
                        CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                "#,
            )
            .bind(&id)
            .bind(&params.tenant_id)
            // The baseline column is `NOT NULL DEFAULT '0'`; binding the Option
            // directly would override that default with NULL for org-less
            // sessions (the request context carries tenant/user only), so the
            // column default is applied here instead.
            .bind(params.organization_id.as_deref().unwrap_or("0"))
            .bind(&params.user_id)
            .bind(&params.conversation_id)
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
                conversation_id: params.conversation_id,
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
                "SELECT id, tenant_id, organization_id, conversation_id, user_id, modality, status, operation_type, source_provider, source_job_id, prompt_preview, favorite, result_count, created_at, updated_at FROM generation_record WHERE id = $1",
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
            // The WHERE clause is assembled from the caller's filters; every
            // clause binds its own placeholder so filter values stay parameter
            // data, never SQL text. The cursor is a keyset pair
            // (`<created_at RFC 3339>|<id>`): the previous implementation
            // accepted a cursor and never applied it, so every page repeated
            // the newest rows.
            let mut binds = ListBinds::default();
            if let Some(conversation_id) = params.conversation_id.as_deref().filter(|v| !v.trim().is_empty()) {
                binds.conversation_id = Some(conversation_id.trim().to_owned());
            }
            if let Some(status) = params.status.as_deref().filter(|v| !v.trim().is_empty()) {
                binds.status = Some(status.trim().to_owned());
            }
            if let Some(modality) = params.modality.as_deref().filter(|v| !v.trim().is_empty()) {
                binds.modality = Some(modality.trim().to_owned());
            }
            if let Some(operation_type) = params.operation_type.as_deref().filter(|v| !v.trim().is_empty()) {
                binds.operation_type = Some(operation_type.trim().to_owned());
            }
            if let Some(q) = params.q.as_deref().filter(|v| !v.trim().is_empty()) {
                binds.q = Some(format!("%{}%", q.trim()));
            }
            if let Some(cursor_id) = params.cursor.as_deref().filter(|v| !v.trim().is_empty()) {
                binds.cursor = Some(decode_list_cursor(cursor_id.trim()).ok_or_else(|| {
                    GenerationsError::InvalidInput("generation list cursor is not a valid keyset token".to_owned())
                })?);
            }
            let sql = assemble_generation_list_sql(
                binds.conversation_id.is_some(),
                binds.status.is_some(),
                binds.modality.is_some(),
                binds.operation_type.is_some(),
                binds.q.is_some(),
                binds.cursor.is_some(),
            );

            // The assembled SQL contains only fixed clause fragments; every
            // filter value binds as a parameter, so the dynamic string carries
            // no injection surface (the audit `AssertSqlSafe` asserts).
            let mut query = sqlx::query_as::<_, GenerationRecordRow>(sqlx::AssertSqlSafe(sql))
                .bind(&params.tenant_id);
            if let Some(value) = binds.conversation_id.as_deref() {
                query = query.bind(value);
            }
            if let Some(value) = binds.status.as_deref() {
                query = query.bind(value);
            }
            if let Some(value) = binds.modality.as_deref() {
                query = query.bind(value);
            }
            if let Some(value) = binds.operation_type.as_deref() {
                query = query.bind(value);
            }
            if let Some(value) = binds.q.as_deref() {
                query = query.bind(value);
            }
            if let Some((created_at, _)) = binds.cursor.as_ref() {
                query = query.bind(created_at);
            }
            if let Some((_, id)) = binds.cursor.as_ref() {
                query = query.bind(id);
            }
            query = query.bind(i64::from(page_size + 1));

            let rows = query.fetch_all(&self.pool).await?;

            let has_more = rows.len() > page_size as usize;
            let taken: Vec<GenerationRecordRow> = rows
                .into_iter()
                .take(page_size as usize)
                .collect();
            let next_cursor = if has_more {
                taken.last().map(|row| encode_list_cursor(row))
            } else {
                None
            };
            let items: Vec<GenerationRecord> = taken
                .into_iter()
                .map(|r| r.into_record())
                .collect();

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
            .bind(chrono::Utc::now())
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
            .bind(chrono::Utc::now())
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
            .bind(chrono::Utc::now())
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
                    source_provider = COALESCE($4, source_provider),
                    result_count = COALESCE($5, result_count),
                    error_code = CASE WHEN $7 THEN NULL ELSE COALESCE($6, error_code) END,
                    error_message = CASE WHEN $7 THEN NULL ELSE COALESCE($8, error_message) END,
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
            .bind(&params.source_provider)
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
        conversation_id: Option<String>,
        user_id: String,
        modality: String,
        status: String,
        operation_type: String,
        source_provider: Option<String>,
        source_job_id: Option<String>,
        prompt_preview: Option<String>,
        favorite: bool,
        result_count: i32,
        created_at: sqlx::types::time::OffsetDateTime,
        updated_at: sqlx::types::time::OffsetDateTime,
    }

    /// Assembles the generation list SQL. Every filter is a fixed clause
    /// fragment and every value binds as a parameter — filter presence only
    /// decides which clauses and placeholder numbers appear, never the text
    /// of a value.
    fn assemble_generation_list_sql(
        conversation_id: bool,
        status: bool,
        modality: bool,
        operation_type: bool,
        q: bool,
        cursor: bool,
    ) -> String {
        let mut sql = String::from(
            "SELECT id, tenant_id, organization_id, conversation_id, user_id, modality, status, operation_type, source_provider, source_job_id, prompt_preview, favorite, result_count, created_at, updated_at FROM generation_record WHERE tenant_id = $1",
        );
        let mut placeholder = 2usize;
        if conversation_id {
            sql.push_str(&format!(" AND conversation_id = ${placeholder}"));
            placeholder += 1;
        }
        if status {
            sql.push_str(&format!(" AND status = ${placeholder}"));
            placeholder += 1;
        }
        if modality {
            sql.push_str(&format!(" AND modality = ${placeholder}"));
            placeholder += 1;
        }
        if operation_type {
            sql.push_str(&format!(" AND operation_type = ${placeholder}"));
            placeholder += 1;
        }
        if q {
            sql.push_str(&format!(" AND prompt_preview ILIKE ${placeholder}"));
            placeholder += 1;
        }
        if cursor {
            sql.push_str(&format!(
                " AND (created_at, id) < (${placeholder}::timestamptz, ${cursor_placeholder})",
                placeholder = placeholder,
                cursor_placeholder = placeholder + 1,
            ));
            placeholder += 2;
        }
        sql.push_str(" ORDER BY created_at DESC, id DESC");
        sql.push_str(&format!(" LIMIT ${placeholder}"));
        sql
    }

    /// Filter values collected before the list SQL is assembled, so clause
    /// text and bind order cannot drift apart.
    #[derive(Default)]
    struct ListBinds {
        conversation_id: Option<String>,
        status: Option<String>,
        modality: Option<String>,
        operation_type: Option<String>,
        q: Option<String>,
        cursor: Option<(time::OffsetDateTime, String)>,
    }

    /// Encodes the keyset cursor as `<created_at RFC 3339>|<record id>`.
    fn encode_list_cursor(row: &GenerationRecordRow) -> String {
        format!(
            "{}|{}",
            row.created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            row.id
        )
    }

    /// Decodes a keyset cursor; `None` marks a malformed token.
    fn decode_list_cursor(cursor: &str) -> Option<(sqlx::types::time::OffsetDateTime, String)> {
        let (created_at, id) = cursor.split_once('|')?;
        let created_at = time::OffsetDateTime::parse(
            created_at,
            &time::format_description::well_known::Rfc3339,
        )
        .ok()?;
        let id = id.trim().to_owned();
        (!id.is_empty()).then_some((created_at, id))
    }

    impl GenerationRecordRow {
        fn into_record(self) -> GenerationRecord {
            GenerationRecord {
                id: self.id,
                tenant_id: self.tenant_id,
                organization_id: self.organization_id,
                conversation_id: self.conversation_id,
                user_id: self.user_id,
                modality: GenerationModality::parse(&self.modality).unwrap_or(GenerationModality::Image),
                status: GenerationStatus::parse(&self.status).unwrap_or(GenerationStatus::Queued),
                operation_type: self.operation_type,
                source_provider: self.source_provider,
                source_job_id: self.source_job_id,
                prompt_preview: self.prompt_preview,
                favorite: self.favorite,
                result_count: self.result_count,
                created_at: rfc3339_timestamp(self.created_at),
                updated_at: rfc3339_timestamp(self.updated_at),
            }
        }
    }

    #[test]
    fn list_sql_without_filters_binds_tenant_and_limit_only() {
        let sql = assemble_generation_list_sql(false, false, false, false, false, false);
        assert!(sql.ends_with("ORDER BY created_at DESC, id DESC LIMIT $2"));
        assert!(!sql.contains(" AND "));
    }

    #[test]
    fn list_sql_numbers_every_filter_clause_in_bind_order() {
        let sql = assemble_generation_list_sql(true, true, true, true, true, true);
        assert!(sql.contains(" AND conversation_id = $2"));
        assert!(sql.contains(" AND status = $3"));
        assert!(sql.contains(" AND modality = $4"));
        assert!(sql.contains(" AND operation_type = $5"));
        assert!(sql.contains(" AND prompt_preview ILIKE $6"));
        assert!(sql.contains(" AND (created_at, id) < ($7::timestamptz, $8)"));
        assert!(sql.ends_with("ORDER BY created_at DESC, id DESC LIMIT $9"));
    }

    #[test]
    fn list_sql_keeps_clauses_absent_when_their_filter_is_off() {
        let sql = assemble_generation_list_sql(false, true, false, false, true, false);
        assert!(!sql.contains("conversation_id ="));
        assert!(sql.contains(" AND status = $2"));
        assert!(!sql.contains(" AND modality"));
        assert!(sql.contains(" AND prompt_preview ILIKE $3"));
        assert!(sql.ends_with("LIMIT $4"));
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
        created_at: sqlx::types::time::OffsetDateTime,
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
                created_at: rfc3339_timestamp(self.created_at),
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
        created_at: sqlx::types::time::OffsetDateTime,
    }

    impl TimelineEventRow {
        fn into_event(self) -> GenerationTimelineEvent {
            GenerationTimelineEvent {
                id: self.id,
                generation_id: self.generation_id,
                event_type: self.event_type,
                message: self.message,
                payload: self.payload,
                created_at: rfc3339_timestamp(self.created_at),
            }
        }
    }

    /// Renders a baseline `TIMESTAMPTZ` column as the RFC 3339 text the
    /// generation domain models carry. The row structs cannot bind `String`
    /// directly: Postgres reports the column as TIMESTAMPTZ and sqlx (with the
    /// `time` feature) only decodes that into `time::OffsetDateTime`.
    fn rfc3339_timestamp(value: sqlx::types::time::OffsetDateTime) -> String {
        chrono::DateTime::from_timestamp(
            value.unix_timestamp(),
            value.nanosecond() % 1_000_000_000,
        )
        .map(|datetime| datetime.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(|| value.to_string())
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

/// Canonical Web Module definition for this application
/// (API_ASSEMBLY_SPEC §4.1.1): the complete HTTP surface — every route,
/// manifest, and OpenAPI document of this owner — as one installable module.
pub async fn web_module() -> Result<WebModule, String> {
    Ok(WebModule::from_contribution(assemble_api_router().await?))
}

/// Same as [`web_module`] but composed on a process-shared database pool
/// (platform gateways, API_ASSEMBLY_SPEC §4.1.1).
pub async fn web_module_with_pool(pool: DatabasePool) -> Result<WebModule, String> {
    Ok(WebModule::from_contribution(assemble_api_router_with_pool(pool).await?))
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
