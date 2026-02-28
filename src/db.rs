use std::{fs::OpenOptions, path::Path};

use sqlx::SqlitePool;

use crate::error::AppError;

pub async fn connect(database_url: &str) -> Result<SqlitePool, AppError> {
    ensure_sqlite_file_ready(database_url)?;
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(8)
        .connect(database_url)
        .await?;
    init_schema(&pool).await?;
    Ok(pool)
}

fn ensure_sqlite_file_ready(database_url: &str) -> Result<(), AppError> {
    let trimmed = database_url.trim();
    if !trimmed.starts_with("sqlite:") {
        return Ok(());
    }

    // Supports sqlite://path, sqlite:///abs/path and sqlite:path formats.
    let raw_path = if let Some(v) = trimmed.strip_prefix("sqlite://") {
        v
    } else if let Some(v) = trimmed.strip_prefix("sqlite:") {
        v
    } else {
        return Ok(());
    };

    let raw_path = raw_path
        .split('?')
        .next()
        .unwrap_or(raw_path)
        .split('#')
        .next()
        .unwrap_or(raw_path);

    if raw_path == ":memory:" {
        return Ok(());
    }

    // Handle sqlite:///C:/... style absolute path on Windows.
    let path = if raw_path.starts_with('/') && raw_path.len() >= 3 && raw_path.as_bytes()[2] == b':'
    {
        &raw_path[1..]
    } else {
        raw_path
    };

    if path.is_empty() {
        return Ok(());
    }

    let db_path = Path::new(path);
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::internal(format!("failed to create database dir {parent:?}: {e}"))
            })?;
        }
    }

    if !db_path.exists() {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(db_path)
            .map_err(|e| {
                AppError::internal(format!("failed to create database file {db_path:?}: {e}"))
            })?;
    }

    Ok(())
}

async fn init_schema(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_keys (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            api_key TEXT NOT NULL UNIQUE,
            token_limit INTEGER NOT NULL,
            consumed_tokens INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL,
            disabled_reason TEXT NULL,
            notes TEXT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            disabled_at INTEGER NULL,
            deleted_at INTEGER NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS usage_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            api_key_id TEXT NOT NULL,
            request_id TEXT NULL,
            path TEXT NOT NULL,
            model TEXT NULL,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            reasoning_tokens INTEGER NOT NULL DEFAULT 0,
            cached_tokens INTEGER NOT NULL DEFAULT 0,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            upstream_status INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(api_key_id) REFERENCES api_keys(id)
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_status ON api_keys(status);")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_updated_at ON api_keys(updated_at DESC);")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_usage_events_key_time ON usage_events(api_key_id, created_at DESC);",
    )
    .execute(pool)
    .await?;

    Ok(())
}
