use askama::Template;
use axum::{Form, Router, extract::State, response::IntoResponse, routing::get};
use pulldown_cmark::{Options, Parser, html};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::{config::AppConfig, state::AppState};

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
    notice_html: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeyCheckForm {
    api_key: String,
}

async fn key_check_page(State(state): State<AppState>) -> impl IntoResponse {
    build_template(&state, String::new(), None, None).await
}

async fn key_check_submit(
    State(state): State<AppState>,
    Form(form): Form<KeyCheckForm>,
) -> impl IntoResponse {
    let query_key = normalize_api_key(&form.api_key);
    if query_key.is_empty() {
        return build_template(
            &state,
            query_key,
            None,
            Some("请输入有效的 API Key".to_string()),
        )
        .await;
    }

    match state.repo.find_by_api_key(&query_key).await {
        Ok(Some(row)) => {
            let result = PublicKeyBalanceView {
                status: row.status,
                token_limit: row.token_limit,
                consumed_tokens: row.consumed_tokens,
                remaining_tokens: compute_remaining_tokens(row.token_limit, row.consumed_tokens),
            };
            build_template(&state, query_key, Some(result), None).await
        }
        Ok(None) => build_template(&state, query_key, None, Some("未找到该 Key".to_string())).await,
        Err(_) => {
            build_template(
                &state,
                query_key,
                None,
                Some("查询失败，请稍后重试".to_string()),
            )
            .await
        }
    }
}

async fn build_template(
    state: &AppState,
    query_key: String,
    result: Option<PublicKeyBalanceView>,
    err: Option<String>,
) -> PublicKeyCheckTemplate {
    PublicKeyCheckTemplate {
        query_key,
        result,
        err,
        notice_html: load_notice_markdown_html(&state.config).await,
    }
}

async fn load_notice_markdown_html(config: &AppConfig) -> Option<String> {
    let raw_path = config.public_notice_file.as_deref()?.trim();
    if raw_path.is_empty() {
        return None;
    }

    let path = resolve_notice_path(raw_path);
    let markdown = tokio::fs::read_to_string(path).await.ok()?;
    let markdown = markdown.trim();
    if markdown.is_empty() {
        return None;
    }

    let rendered = render_markdown_to_safe_html(markdown);
    if rendered.trim().is_empty() {
        None
    } else {
        Some(rendered)
    }
}

fn resolve_notice_path(raw: &str) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    match std::env::current_dir() {
        Ok(dir) => dir.join(path),
        Err(_) => path.to_path_buf(),
    }
}

fn render_markdown_to_safe_html(markdown: &str) -> String {
    let mut html_output = String::new();
    let parser = Parser::new_ext(markdown, markdown_options());
    html::push_html(&mut html_output, parser);
    ammonia::Builder::default()
        .link_rel(Some("noopener noreferrer"))
        .clean(&html_output)
        .to_string()
}

fn markdown_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options
}

fn normalize_api_key(value: &str) -> String {
    value.trim().to_string()
}

fn compute_remaining_tokens(token_limit: i64, consumed_tokens: i64) -> i64 {
    (token_limit - consumed_tokens).max(0)
}

#[cfg(test)]
mod tests {
    use super::{
        compute_remaining_tokens, normalize_api_key, render_markdown_to_safe_html,
        resolve_notice_path,
    };

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

    #[test]
    fn relative_notice_path_resolves_against_current_dir() {
        let path = resolve_notice_path("./data/public_notice.md");
        assert!(path.ends_with("data/public_notice.md"));
    }

    #[test]
    fn absolute_notice_path_is_kept() {
        let abs = std::env::current_dir()
            .expect("current dir")
            .join("public_notice.md");
        let resolved = resolve_notice_path(abs.to_str().expect("path to str"));
        assert_eq!(resolved, abs);
    }

    #[test]
    fn markdown_output_is_sanitized() {
        let html = render_markdown_to_safe_html("hi<script>alert(1)</script>");
        assert!(html.contains("<p>hi</p>"));
        assert!(!html.contains("<script>"));
    }
}
