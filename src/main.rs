mod admin_api;
mod admin_auth;
mod admin_page;
mod config;
mod db;
mod error;
mod keygen;
mod models;
mod proxy;
mod public_query_page;
mod repository;
mod state;
mod static_assets;
mod token_usage;

use std::sync::Arc;

use axum::{Router, middleware, routing::get};
use config::AppConfig;
use error::AppError;
use repository::Repository;
use state::AppState;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = Arc::new(AppConfig::from_env()?);
    let pool = db::connect(&config.database_url).await?;
    let repo = Repository::new(pool);
    let http_client = reqwest::Client::builder()
        .tcp_keepalive(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::internal(format!("failed to init reqwest client: {e}")))?;

    let state = AppState {
        config: config.clone(),
        repo,
        http_client,
    };
    let admin_page_layer =
        middleware::from_fn_with_state(state.clone(), admin_auth::require_basic_auth);
    let admin_api_layer =
        middleware::from_fn_with_state(state.clone(), admin_auth::require_basic_auth);

    let app = Router::new()
        .route("/health", get(health))
        .merge(public_query_page::router())
        .merge(admin_page::router().route_layer(admin_page_layer))
        .nest(
            "/admin/api",
            admin_api::router().route_layer(admin_api_layer),
        )
        .merge(proxy::router())
        .route("/assets/*path", get(static_assets::serve_asset))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .map_err(|e| AppError::internal(format!("failed to bind {}: {e}", config.bind_addr)))?;

    info!(
        bind_addr = %config.bind_addr,
        upstream = %config.upstream_base_url,
        database = %config.database_url,
        notice_file = config.public_notice_file.as_deref().unwrap_or(""),
        "CLIProxyAPI-admin started"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| AppError::internal(format!("server error: {e}")))?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
            let _ = sigterm.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("shutdown signal received");
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,tower_http=info,sqlx=warn".into());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
