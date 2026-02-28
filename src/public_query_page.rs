use askama::Template;
use axum::{
    Form, Router,
    extract::State,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/key-check", get(key_check_page).post(key_check_submit))
}

#[derive(Debug, Clone)]
struct PublicKeyBalanceView {
    status: String,
    token_limit: i64,
    consumed_tokens: i64,
    remaining_tokens: i64,
}

#[derive(Template)]
#[template(path = "public_key_check.html")]
struct PublicKeyCheckTemplate {
    query_key: String,
    result: Option<PublicKeyBalanceView>,
    err: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeyCheckForm {
    api_key: String,
}

async fn key_check_page() -> impl IntoResponse {
    PublicKeyCheckTemplate {
        query_key: String::new(),
        result: None,
        err: None,
    }
}

async fn key_check_submit(
    State(state): State<AppState>,
    Form(form): Form<KeyCheckForm>,
) -> impl IntoResponse {
    let query_key = normalize_api_key(&form.api_key);
    if query_key.is_empty() {
        return PublicKeyCheckTemplate {
            query_key,
            result: None,
            err: Some("请输入有效的 API Key".to_string()),
        };
    }

    match state.repo.find_by_api_key(&query_key).await {
        Ok(Some(row)) => {
            let result = PublicKeyBalanceView {
                status: row.status,
                token_limit: row.token_limit,
                consumed_tokens: row.consumed_tokens,
                remaining_tokens: compute_remaining_tokens(row.token_limit, row.consumed_tokens),
            };
            PublicKeyCheckTemplate {
                query_key,
                result: Some(result),
                err: None,
            }
        }
        Ok(None) => PublicKeyCheckTemplate {
            query_key,
            result: None,
            err: Some("未找到该 Key".to_string()),
        },
        Err(_) => PublicKeyCheckTemplate {
            query_key,
            result: None,
            err: Some("查询失败，请稍后重试".to_string()),
        },
    }
}

fn normalize_api_key(value: &str) -> String {
    value.trim().to_string()
}

fn compute_remaining_tokens(token_limit: i64, consumed_tokens: i64) -> i64 {
    (token_limit - consumed_tokens).max(0)
}

#[cfg(test)]
mod tests {
    use super::{compute_remaining_tokens, normalize_api_key};

    #[test]
    fn normalize_api_key_trims_spaces() {
        let key = normalize_api_key("  sk-demo-key  ");
        assert_eq!(key, "sk-demo-key");
    }

    #[test]
    fn compute_remaining_tokens_clamps_to_zero() {
        assert_eq!(compute_remaining_tokens(100, 40), 60);
        assert_eq!(compute_remaining_tokens(100, 100), 0);
        assert_eq!(compute_remaining_tokens(100, 130), 0);
    }
}
