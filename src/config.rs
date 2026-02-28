use std::env;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub upstream_base_url: String,
    pub upstream_bearer_key: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8318".to_string());
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://./data/admin.db".to_string());
        let upstream_base_url =
            env::var("UPSTREAM_BASE_URL").unwrap_or_else(|_| "http://localhost:8317".to_string());
        let upstream_bearer_key = env::var("UPSTREAM_BEARER_KEY").map_err(|_| {
            AppError::config(
                "missing env UPSTREAM_BEARER_KEY. set it in .env or process environment",
            )
        })?;

        let upstream_bearer_key = upstream_bearer_key.trim().to_string();
        if upstream_bearer_key.is_empty() {
            return Err(AppError::config("UPSTREAM_BEARER_KEY cannot be empty"));
        }

        Ok(Self {
            bind_addr,
            database_url,
            upstream_base_url: upstream_base_url.trim_end_matches('/').to_string(),
            upstream_bearer_key,
        })
    }
}
