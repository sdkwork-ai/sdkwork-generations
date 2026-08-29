//! HTTP handlers for the generations service.
//!
//! Implements all 19 endpoints exposed under `/app/v3/api/generations`.

use axum::extract::{Extension, Path, Query, State};
use serde_json::Value;

use crate::context::GenerationsRequestContext;
use crate::domain::models::{
    CreateGenerationCommandRequest, FavoriteGenerationRequest, GenerationActionRequest,
    SaveGenerationResultToAssetsRequest,
};
use crate::error::GenerationsError;
use crate::ports::{ListResultsParams, ListTimelineParams};
use crate::service::generations_service::{GenerationsService, GenerationsServiceState};

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the full app-api router for the generations service.
pub fn build_app_routes() -> axum::Router<GenerationsServiceState> {
    axum::Router::new()
        // List
        .route(
            "/app/v3/api/generations",
            axum::routing::get(list_generations),
        )
        // Image generations
        .route(
            "/app/v3/api/generations/images/text_to_image",
            axum::routing::post(create_text_to_image),
        )
        .route(
            "/app/v3/api/generations/images/image_edit",
            axum::routing::post(create_image_edit),
        )
        // Video generations
        .route(
            "/app/v3/api/generations/videos/text_to_video",
            axum::routing::post(create_text_to_video),
        )
        .route(
            "/app/v3/api/generations/videos/image_to_video",
            axum::routing::post(create_image_to_video),
        )
        .route(
            "/app/v3/api/generations/videos/video_extend",
            axum::routing::post(create_video_extend),
        )
        // Music generations
        .route(
            "/app/v3/api/generations/music/text_to_music",
            axum::routing::post(create_text_to_music),
        )
        .route(
            "/app/v3/api/generations/music/lyrics_to_music",
            axum::routing::post(create_lyrics_to_music),
        )
        // Sound effects
        .route(
            "/app/v3/api/generations/sound_effects",
            axum::routing::post(create_sound_effects),
        )
        // Voice
        .route(
            "/app/v3/api/generations/voice/speech",
            axum::routing::post(create_speech),
        )
        .route(
            "/app/v3/api/generations/voice/transcription",
            axum::routing::post(create_transcription),
        )
        .route(
            "/app/v3/api/generations/voice/translation",
            axum::routing::post(create_translation),
        )
        // Single record
        .route(
            "/app/v3/api/generations/{generationId}",
            axum::routing::get(get_generation),
        )
        // Results
        .route(
            "/app/v3/api/generations/{generationId}/results",
            axum::routing::get(list_results),
        )
        // Timeline
        .route(
            "/app/v3/api/generations/{generationId}/timeline",
            axum::routing::get(list_timeline),
        )
        // Actions
        .route(
            "/app/v3/api/generations/{generationId}/cancel",
            axum::routing::post(cancel_generation),
        )
        .route(
            "/app/v3/api/generations/{generationId}/retry",
            axum::routing::post(retry_generation),
        )
        .route(
            "/app/v3/api/generations/{generationId}/favorite",
            axum::routing::post(favorite_generation),
        )
        // Save to assets
        .route(
            "/app/v3/api/generations/{generationId}/results/{resultId}/save_to_assets",
            axum::routing::post(save_to_assets),
        )
}

// ---------------------------------------------------------------------------
// Request / response helpers
// ---------------------------------------------------------------------------

fn trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn success(data: Value) -> axum::Json<Value> {
    axum::Json(serde_json::json!({
        "code": 0,
        "data": data,
        "traceId": trace_id()
    }))
}

fn failure(error: &GenerationsError) -> axum::Json<Value> {
    axum::Json(serde_json::json!({
        "code": error.platform_code(),
        "message": error.to_string(),
        "traceId": trace_id()
    }))
}

/// Extract the request context from the request extension.
fn context_from_extension(context: &GenerationsRequestContext) -> GenerationsRequestContext {
    context.clone()
}

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
pub struct ListGenerationsQuery {
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<i32>,
    pub status: Option<String>,
    pub modality: Option<String>,
    #[serde(rename = "operation_type")]
    pub operation_type: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListResultsQuery {
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListTimelineQuery {
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<i32>,
}

// ---------------------------------------------------------------------------
// Endpoint handlers
// ---------------------------------------------------------------------------

/// GET /app/v3/api/generations — List generation records.
async fn list_generations(
    State(state): State<GenerationsServiceState>,
    Query(query): Query<ListGenerationsQuery>,
    Extension(extension): Extension<GenerationsRequestContext>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    let params = crate::ports::ListGenerationsParams {
        tenant_id: context.http.tenant_id.clone(),
        cursor: query.cursor,
        page_size: query.page_size,
        status: query.status,
        modality: query.modality,
        operation_type: query.operation_type,
        q: query.q,
    };

    match GenerationsService::list_generations(&state, context.http.tenant_id.clone(), params).await {
        Ok((items, page_info)) => {
            success(serde_json::json!({ "items": items, "pageInfo": page_info }))
        }
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/images/text_to_image
async fn create_text_to_image(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_text_to_image(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/images/image_edit
async fn create_image_edit(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_image_edit(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/videos/text_to_video
async fn create_text_to_video(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_text_to_video(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/videos/image_to_video
async fn create_image_to_video(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_image_to_video(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/videos/video_extend
async fn create_video_extend(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_video_extend(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/music/text_to_music
async fn create_text_to_music(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_text_to_music(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/music/lyrics_to_music
async fn create_lyrics_to_music(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_lyrics_to_music(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/sound_effects
async fn create_sound_effects(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_sound_effects(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/voice/speech
async fn create_speech(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_speech(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/voice/transcription
async fn create_transcription(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_transcription(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/voice/translation
async fn create_translation(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    axum::Json(body): axum::Json<CreateGenerationCommandRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::create_translation(&state, &context, &body).await {
        Ok(resp) => success(serde_json::json!({ "item": resp.generation })),
        Err(error) => failure(&error),
    }
}

/// GET /app/v3/api/generations/{generationId}
async fn get_generation(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    Path(generation_id): Path<String>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::get_generation(&state, &context, &generation_id).await {
        Ok(record) => success(serde_json::json!({ "item": record })),
        Err(error) => failure(&error),
    }
}

/// GET /app/v3/api/generations/{generationId}/results
async fn list_results(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    Path(generation_id): Path<String>,
    Query(query): Query<ListResultsQuery>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    let params = ListResultsParams {
        generation_id: generation_id.clone(),
        cursor: query.cursor,
        page_size: query.page_size,
    };

    match GenerationsService::list_results(&state, &context, &generation_id, params).await {
        Ok((items, page_info)) => {
            success(serde_json::json!({ "items": items, "pageInfo": page_info }))
        }
        Err(error) => failure(&error),
    }
}

/// GET /app/v3/api/generations/{generationId}/timeline
async fn list_timeline(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    Path(generation_id): Path<String>,
    Query(query): Query<ListTimelineQuery>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    let params = ListTimelineParams {
        generation_id: generation_id.clone(),
        cursor: query.cursor,
        page_size: query.page_size,
    };

    if let Some(record) = state.repository().get(&generation_id).await.ok().flatten() {
        crate::service::generations_service::refresh_pending_generation(&state, &context, record)
            .await;
    }

    match GenerationsService::list_timeline(&state, &generation_id, params).await {
        Ok((items, page_info)) => {
            success(serde_json::json!({ "items": items, "pageInfo": page_info }))
        }
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/{generationId}/cancel
async fn cancel_generation(
    State(state): State<GenerationsServiceState>,
    Path(generation_id): Path<String>,
    axum::Json(body): axum::Json<GenerationActionRequest>,
) -> axum::Json<Value> {
    match GenerationsService::cancel_generation(&state, &generation_id, body.reason.as_deref())
        .await
    {
        Ok(record) => success(serde_json::json!({ "item": record })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/{generationId}/retry
async fn retry_generation(
    State(state): State<GenerationsServiceState>,
    Path(generation_id): Path<String>,
    axum::Json(body): axum::Json<GenerationActionRequest>,
) -> axum::Json<Value> {
    match GenerationsService::retry_generation(&state, &generation_id, body.reason.as_deref())
        .await
    {
        Ok(record) => success(serde_json::json!({ "item": record })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/{generationId}/favorite
async fn favorite_generation(
    State(state): State<GenerationsServiceState>,
    Path(generation_id): Path<String>,
    axum::Json(body): axum::Json<FavoriteGenerationRequest>,
) -> axum::Json<Value> {
    match GenerationsService::favorite_generation(&state, &generation_id, &body).await {
        Ok(record) => success(serde_json::json!({ "item": record })),
        Err(error) => failure(&error),
    }
}

/// POST /app/v3/api/generations/{generationId}/results/{resultId}/save_to_assets
async fn save_to_assets(
    State(state): State<GenerationsServiceState>,
    Extension(extension): Extension<GenerationsRequestContext>,
    Path((generation_id, result_id)): Path<(String, String)>,
    axum::Json(body): axum::Json<SaveGenerationResultToAssetsRequest>,
) -> axum::Json<Value> {
    let context = context_from_extension(&extension);
    match GenerationsService::save_to_assets(&state, &context, &generation_id, &result_id, &body)
        .await
    {
        Ok(result) => success(serde_json::json!({ "item": result })),
        Err(error) => failure(&error),
    }
}
