use axum::{
    Router,
    body::{Body, Bytes},
    extract::{OriginalUri, State},
    http::{HeaderMap, Method, Response, StatusCode},
    response::IntoResponse,
    routing::{any, get, post},
};
use ulid::Ulid;

use crate::{
    error::AppError,
    models::ApiKeyStatus,
    state::AppState,
    token_usage::{extract_model_from_request, extract_model_from_response, extract_usage},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/models", get(proxy_handler))
        .route("/v1/chat/completions", post(proxy_handler))
        .route("/v1/completions", post(proxy_handler))
        .route(
            "/v1/responses",
            get(responses_websocket_not_supported).post(proxy_handler),
        )
        .route("/v1/*path", any(v1_not_supported))
}

async fn responses_websocket_not_supported() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(serde_json::json!({
            "error": "GET /v1/responses websocket is not supported in CLIProxyAPI-admin",
        })),
    )
}

async fn v1_not_supported(OriginalUri(uri): OriginalUri) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "error": format!("unsupported endpoint: {}", uri.path()),
        })),
    )
}

async fn proxy_handler(
    State(state): State<AppState>,
    method: Method,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, AppError> {
    let client_key = extract_bearer_token(&headers)
        .ok_or_else(|| AppError::unauthorized("missing Authorization Bearer <api_key>"))?;

    let key_record = state
        .repo
        .find_by_api_key(client_key)
        .await?
        .ok_or_else(|| AppError::unauthorized("invalid api key"))?;

    if key_record.status != ApiKeyStatus::Active.as_str() {
        return Err(AppError::forbidden("api key is disabled"));
    }
    if key_record.consumed_tokens >= key_record.token_limit {
        state.repo.disable_key_quota(&key_record.id).await?;
        return Err(AppError::forbidden(
            "api key quota exhausted and has been disabled",
        ));
    }

    let path = uri.path().to_string();
    let upstream_url = build_upstream_url(&state.config.upstream_base_url, &uri);
    let mut request_builder = state.http_client.request(method.clone(), upstream_url);
    for (name, value) in &headers {
        if should_skip_request_header(name.as_str()) {
            continue;
        }
        request_builder = request_builder.header(name.as_str(), value.clone());
    }
    request_builder = request_builder.bearer_auth(&state.config.upstream_bearer_key);
    if method != Method::GET && method != Method::HEAD && !body.is_empty() {
        request_builder = request_builder.body(body.clone());
    }

    let upstream_response = request_builder.send().await?;
    let status = upstream_response.status();
    let response_headers = upstream_response.headers().clone();
    let content_type = response_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let response_bytes = upstream_response.bytes().await?;

    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| Ulid::new().to_string().to_lowercase());

    let usage = extract_usage(&response_bytes, content_type.as_deref());
    let model = extract_model_from_request(&body)
        .or_else(|| extract_model_from_response(&response_bytes, content_type.as_deref()));

    state
        .repo
        .record_usage_and_consume(
            &key_record.id,
            usage,
            &path,
            model.as_deref(),
            status.as_u16() as i64,
            Some(&request_id),
        )
        .await?;

    let mut response_builder = Response::builder().status(status);
    for (name, value) in &response_headers {
        if should_skip_response_header(name.as_str()) {
            continue;
        }
        response_builder = response_builder.header(name, value);
    }
    let response = response_builder
        .body(Body::from(response_bytes.to_vec()))
        .map_err(|e| AppError::internal(format!("failed to build response: {e}")))?;
    Ok(response)
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let raw = headers.get("authorization")?.to_str().ok()?;
    let mut parts = raw.splitn(2, ' ');
    let scheme = parts.next()?;
    let token = parts.next()?.trim();
    if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() {
        return None;
    }
    Some(token)
}

fn build_upstream_url(base_url: &str, uri: &http::Uri) -> String {
    if let Some(query) = uri.query() {
        format!("{base_url}{}?{query}", uri.path())
    } else {
        format!("{base_url}{}", uri.path())
    }
}

fn should_skip_request_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "host"
            | "content-length"
            | "authorization"
            | "connection"
            | "proxy-connection"
            | "upgrade"
            | "keep-alive"
            | "transfer-encoding"
    )
}

fn should_skip_response_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "content-length" | "connection" | "transfer-encoding"
    )
}
