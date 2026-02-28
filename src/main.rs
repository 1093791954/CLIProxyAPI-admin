mod admin_api;
mod admin_page;
mod config;
mod db;
mod error;
mod keygen;
mod models;
mod proxy;
mod repository;
mod state;
mod token_usage;

use std::sync::Arc;

use axum::{Router, routing::get};
use config::AppConfig;
use error::AppError;
use repository::Repository;
use state::AppState;
use tower_http::{services::ServeDir, trace::TraceLayer};
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

    let app = Router::new()
        .route("/health", get(health))
        .merge(admin_page::router())
        .nest("/admin/api", admin_api::router())
        .merge(proxy::router())
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .map_err(|e| AppError::internal(format!("failed to bind {}: {e}", config.bind_addr)))?;

    info!(
        bind_addr = %config.bind_addr,
        upstream = %config.upstream_base_url,
        database = %config.database_url,
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
