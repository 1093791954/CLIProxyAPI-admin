use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::Deserialize;

use crate::{
    keygen,
    models::{
        ApiKeyDto, CreateKeyForm, CreateKeyRequest, ListFilter, ListKeysQuery, PatchKeyRequest,
        UpdateKeyForm, UsageEventDto,
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(root_redirect))
        .route("/admin", get(dashboard))
        .route("/admin/keys/:id", get(key_detail))
        .route("/admin/actions/create", post(create_action))
        .route("/admin/actions/:id/update", post(update_action))
        .route("/admin/actions/:id/disable", post(disable_action))
        .route("/admin/actions/:id/enable", post(enable_action))
        .route("/admin/actions/:id/reset", post(reset_action))
        .route("/admin/actions/:id/delete", post(delete_action))
}

async fn root_redirect() -> Redirect {
    Redirect::to("/admin")
}

#[derive(Debug, Deserialize)]
struct DashboardQuery {
    status: Option<String>,
    search: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
    msg: Option<String>,
    err: Option<String>,
    new_key: Option<String>,
}

#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct DashboardTemplate {
    keys: Vec<ApiKeyDto>,
    status: String,
    search: String,
    page: u32,
    page_size: u32,
    total: i64,
    msg: Option<String>,
    err: Option<String>,
    new_key: Option<String>,
}

async fn dashboard(
    State(state): State<AppState>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let list_query = ListKeysQuery {
        status: query.status.clone(),
        search: query.search.clone(),
        page: query.page,
        page_size: query.page_size,
    };
    let filter = ListFilter::from_query(&list_query);
    let (items, total) = match state.repo.list_keys(&filter).await {
        Ok(v) => v,
        Err(_) => (Vec::new(), 0),
    };
    let template = DashboardTemplate {
        keys: items.into_iter().map(ApiKeyDto::from).collect(),
        status: query.status.unwrap_or_default(),
        search: query.search.unwrap_or_default(),
        page: filter.page,
        page_size: filter.page_size,
        total,
        msg: query.msg,
        err: query.err,
        new_key: query.new_key,
    };
    template
}

#[derive(Template)]
#[template(path = "admin_key_detail.html")]
struct KeyDetailTemplate {
    key: ApiKeyDto,
    events: Vec<UsageEventDto>,
    msg: Option<String>,
    err: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DetailQuery {
    msg: Option<String>,
    err: Option<String>,
}

async fn key_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DetailQuery>,
) -> impl IntoResponse {
    let key = match state.repo.get_key_by_id(&id).await {
        Ok(v) => v,
        Err(_) => return Redirect::to("/admin?err=key_not_found").into_response(),
    };
    let events = state
        .repo
        .list_usage_events(&id, 100)
        .await
        .unwrap_or_default();
    let template = KeyDetailTemplate {
        key: ApiKeyDto::from(key),
        events: events.into_iter().map(UsageEventDto::from).collect(),
        msg: query.msg,
        err: query.err,
    };
    template.into_response()
}

async fn create_action(
    State(state): State<AppState>,
    Form(form): Form<CreateKeyForm>,
) -> impl IntoResponse {
    let req = CreateKeyRequest {
        name: form.name.trim().to_string(),
        token_limit: form.token_limit,
        notes: form.notes.clone(),
    };
    if req.name.is_empty() || req.token_limit <= 0 {
        return Redirect::to("/admin?err=invalid_create_form");
    }

    let id = keygen::new_key_id();
    let api_key = keygen::new_api_key();
    match state
        .repo
        .create_key(
            &id,
            &req.name,
            &api_key,
            req.token_limit,
            req.notes.as_deref(),
        )
        .await
    {
        Ok(_) => Redirect::to(&format!("/admin?msg=created&new_key={api_key}")),
        Err(_) => Redirect::to("/admin?err=create_failed"),
    }
}

async fn update_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<UpdateKeyForm>,
) -> impl IntoResponse {
    let payload = PatchKeyRequest {
        name: Some(form.name),
        token_limit: Some(form.token_limit),
        notes: Some(form.notes.unwrap_or_default()),
        status: None,
    };
    match state.repo.patch_key(&id, payload).await {
        Ok(_) => Redirect::to("/admin?msg=updated"),
        Err(_) => Redirect::to("/admin?err=update_failed"),
    }
}

async fn disable_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.repo.disable_key_manual(&id).await {
        Ok(_) => Redirect::to("/admin?msg=disabled"),
        Err(_) => Redirect::to("/admin?err=disable_failed"),
    }
}

async fn enable_action(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.repo.enable_key(&id).await {
        Ok(_) => Redirect::to("/admin?msg=enabled"),
        Err(_) => Redirect::to("/admin?err=enable_failed"),
    }
}

async fn reset_action(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.repo.reset_usage(&id).await {
        Ok(_) => Redirect::to("/admin?msg=usage_reset"),
        Err(_) => Redirect::to("/admin?err=reset_failed"),
    }
}

async fn delete_action(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.repo.soft_delete_key(&id).await {
        Ok(_) => Redirect::to("/admin?msg=deleted"),
        Err(_) => Redirect::to("/admin?err=delete_failed"),
    }
}
