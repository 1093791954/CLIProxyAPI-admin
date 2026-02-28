use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, patch, post},
};
use serde::Deserialize;

use crate::{
    error::AppError,
    keygen,
    models::{
        ApiKeyDto, CreateKeyRequest, ListFilter, ListKeysQuery, ListKeysResponse, PatchKeyRequest,
        UsageEventDto,
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/keys", post(create_key).get(list_keys))
        .route("/keys/:id", patch(patch_key).delete(delete_key))
        .route("/keys/:id/disable", post(disable_key))
        .route("/keys/:id/enable", post(enable_key))
        .route("/keys/:id/reset-usage", post(reset_usage))
        .route("/keys/:id/usage-events", get(list_usage_events))
}

async fn create_key(
    State(state): State<AppState>,
    Json(payload): Json<CreateKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::bad_request("name cannot be empty"));
    }
    if payload.token_limit <= 0 {
        return Err(AppError::bad_request("token_limit must be > 0"));
    }

    let id = keygen::new_key_id();
    let api_key = keygen::resolve_api_key(payload.api_key.as_deref())?;
    let row = state
        .repo
        .create_key(
            &id,
            name,
            &api_key,
            payload.token_limit,
            payload.notes.as_deref(),
        )
        .await?;
    Ok(Json(ApiKeyDto::from(row)))
}

async fn list_keys(
    State(state): State<AppState>,
    Query(query): Query<ListKeysQuery>,
) -> Result<impl IntoResponse, AppError> {
    let filter = ListFilter::from_query(&query);
    let (rows, total) = state.repo.list_keys(&filter).await?;
    let items = rows.into_iter().map(ApiKeyDto::from).collect();
    Ok(Json(ListKeysResponse {
        items,
        page: filter.page,
        page_size: filter.page_size,
        total,
    }))
}

async fn patch_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<PatchKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    let row = state.repo.patch_key(&id, payload).await?;
    Ok(Json(ApiKeyDto::from(row)))
}

async fn delete_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    state.repo.soft_delete_key(&id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn disable_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let row = state.repo.disable_key_manual(&id).await?;
    Ok(Json(ApiKeyDto::from(row)))
}

async fn enable_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let row = state.repo.enable_key(&id).await?;
    Ok(Json(ApiKeyDto::from(row)))
}

async fn reset_usage(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let row = state.repo.reset_usage(&id).await?;
    Ok(Json(ApiKeyDto::from(row)))
}

#[derive(Debug, Deserialize)]
struct UsageQuery {
    limit: Option<i64>,
}

async fn list_usage_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<UsageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let limit = query.limit.unwrap_or(100);
    let rows = state.repo.list_usage_events(&id, limit).await?;
    let items: Vec<UsageEventDto> = rows.into_iter().map(UsageEventDto::from).collect();
    Ok(Json(items))
}
