use chrono::Utc;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::{
    error::AppError,
    models::{
        ApiKeyRecord, ApiKeyStatus, ListFilter, PatchKeyRequest, TokenUsage, UsageEventRecord,
    },
};

#[derive(Clone)]
pub struct Repository {
    pool: SqlitePool,
}

impl Repository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_key(
        &self,
        id: &str,
        name: &str,
        api_key: &str,
        token_limit: i64,
        notes: Option<&str>,
    ) -> Result<ApiKeyRecord, AppError> {
        if token_limit <= 0 {
            return Err(AppError::bad_request("token_limit must be > 0"));
        }
        let now = now_ts();
        sqlx::query(
            r#"
            INSERT INTO api_keys (
                id, name, api_key, token_limit, consumed_tokens, status, disabled_reason,
                notes, created_at, updated_at, disabled_at, deleted_at
            ) VALUES (?, ?, ?, ?, 0, 'active', NULL, ?, ?, ?, NULL, NULL)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(api_key)
        .bind(token_limit)
        .bind(notes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_key_by_id(id).await
    }

    pub async fn list_keys(
        &self,
        filter: &ListFilter,
    ) -> Result<(Vec<ApiKeyRecord>, i64), AppError> {
        let mut qb = QueryBuilder::<Sqlite>::new(
            "SELECT id, name, api_key, token_limit, consumed_tokens, status, disabled_reason, notes, created_at, updated_at, disabled_at, deleted_at FROM api_keys WHERE status != 'deleted'",
        );
        apply_filter(&mut qb, filter);
        qb.push(" ORDER BY updated_at DESC LIMIT ");
        qb.push_bind(filter.page_size as i64);
        qb.push(" OFFSET ");
        let offset = (filter.page.saturating_sub(1) as i64) * filter.page_size as i64;
        qb.push_bind(offset);
        let items = qb
            .build_query_as::<ApiKeyRecord>()
            .fetch_all(&self.pool)
            .await?;

        let mut count_qb = QueryBuilder::<Sqlite>::new(
            "SELECT COUNT(1) as cnt FROM api_keys WHERE status != 'deleted'",
        );
        apply_filter(&mut count_qb, filter);
        let total: (i64,) = count_qb.build_query_as().fetch_one(&self.pool).await?;

        Ok((items, total.0))
    }

    pub async fn get_key_by_id(&self, id: &str) -> Result<ApiKeyRecord, AppError> {
        let row = sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT id, name, api_key, token_limit, consumed_tokens, status, disabled_reason, notes, created_at, updated_at, disabled_at, deleted_at
            FROM api_keys
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or_else(|| AppError::not_found("key not found"))
    }

    pub async fn find_by_api_key(&self, api_key: &str) -> Result<Option<ApiKeyRecord>, AppError> {
        let row = sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT id, name, api_key, token_limit, consumed_tokens, status, disabled_reason, notes, created_at, updated_at, disabled_at, deleted_at
            FROM api_keys
            WHERE api_key = ? AND status != 'deleted'
            "#,
        )
        .bind(api_key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn patch_key(
        &self,
        id: &str,
        patch: PatchKeyRequest,
    ) -> Result<ApiKeyRecord, AppError> {
        let mut current = self.get_key_by_id(id).await?;
        if current.status == ApiKeyStatus::Deleted.as_str() {
            return Err(AppError::not_found("key not found"));
        }

        if let Some(name) = patch.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(AppError::bad_request("name cannot be empty"));
            }
            current.name = name.to_string();
        }
        if let Some(token_limit) = patch.token_limit {
            if token_limit <= 0 {
                return Err(AppError::bad_request("token_limit must be > 0"));
            }
            current.token_limit = token_limit;
        }
        if let Some(notes) = patch.notes {
            let notes = notes.trim().to_string();
            current.notes = if notes.is_empty() { None } else { Some(notes) };
        }
        if let Some(status) = patch.status {
            let status = ApiKeyStatus::parse(&status)
                .ok_or_else(|| AppError::bad_request("invalid status, expected active|disabled"))?;
            match status {
                ApiKeyStatus::Active => {
                    if current.consumed_tokens >= current.token_limit {
                        return Err(AppError::bad_request(
                            "key has exhausted quota, reset usage first",
                        ));
                    }
                    current.status = ApiKeyStatus::Active.as_str().to_string();
                    current.disabled_reason = None;
                    current.disabled_at = None;
                }
                ApiKeyStatus::Disabled => {
                    current.status = ApiKeyStatus::Disabled.as_str().to_string();
                    current.disabled_reason = Some("manual".to_string());
                    current.disabled_at = Some(now_ts());
                }
                ApiKeyStatus::Deleted => {
                    return Err(AppError::bad_request(
                        "use delete endpoint for deleted status",
                    ));
                }
            }
        }

        current.updated_at = now_ts();

        sqlx::query(
            r#"
            UPDATE api_keys
            SET name = ?, token_limit = ?, status = ?, disabled_reason = ?, notes = ?, updated_at = ?, disabled_at = ?
            WHERE id = ? AND status != 'deleted'
            "#,
        )
        .bind(&current.name)
        .bind(current.token_limit)
        .bind(&current.status)
        .bind(&current.disabled_reason)
        .bind(&current.notes)
        .bind(current.updated_at)
        .bind(current.disabled_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_key_by_id(id).await
    }

    pub async fn soft_delete_key(&self, id: &str) -> Result<(), AppError> {
        let now = now_ts();
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET status = 'deleted', disabled_reason = 'manual', deleted_at = ?, updated_at = ?, disabled_at = COALESCE(disabled_at, ?)
            WHERE id = ? AND status != 'deleted'
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found("key not found"));
        }
        Ok(())
    }

    pub async fn disable_key_manual(&self, id: &str) -> Result<ApiKeyRecord, AppError> {
        let now = now_ts();
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET status = 'disabled', disabled_reason = 'manual', disabled_at = ?, updated_at = ?
            WHERE id = ? AND status = 'active'
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            let row = self.get_key_by_id(id).await?;
            if row.status == ApiKeyStatus::Deleted.as_str() {
                return Err(AppError::not_found("key not found"));
            }
            return Ok(row);
        }
        self.get_key_by_id(id).await
    }

    pub async fn disable_key_quota(&self, id: &str) -> Result<(), AppError> {
        let now = now_ts();
        sqlx::query(
            r#"
            UPDATE api_keys
            SET status = 'disabled', disabled_reason = 'quota_exceeded', disabled_at = ?, updated_at = ?
            WHERE id = ? AND status = 'active' AND consumed_tokens >= token_limit
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn enable_key(&self, id: &str) -> Result<ApiKeyRecord, AppError> {
        let key = self.get_key_by_id(id).await?;
        if key.status == ApiKeyStatus::Deleted.as_str() {
            return Err(AppError::not_found("key not found"));
        }
        if key.consumed_tokens >= key.token_limit {
            return Err(AppError::bad_request(
                "key has exhausted quota, reset usage first",
            ));
        }
        let now = now_ts();
        sqlx::query(
            r#"
            UPDATE api_keys
            SET status = 'active', disabled_reason = NULL, disabled_at = NULL, updated_at = ?
            WHERE id = ? AND status != 'deleted'
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        self.get_key_by_id(id).await
    }

    pub async fn reset_usage(&self, id: &str) -> Result<ApiKeyRecord, AppError> {
        let now = now_ts();
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET consumed_tokens = 0, updated_at = ?
            WHERE id = ? AND status != 'deleted'
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found("key not found"));
        }
        self.get_key_by_id(id).await
    }

    pub async fn list_usage_events(
        &self,
        key_id: &str,
        limit: i64,
    ) -> Result<Vec<UsageEventRecord>, AppError> {
        let limit = limit.clamp(1, 500);
        let events = sqlx::query_as::<_, UsageEventRecord>(
            r#"
            SELECT id, api_key_id, request_id, path, model, input_tokens, output_tokens, reasoning_tokens, cached_tokens, total_tokens, upstream_status, created_at
            FROM usage_events
            WHERE api_key_id = ?
            ORDER BY created_at DESC, id DESC
            LIMIT ?
            "#,
        )
        .bind(key_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(events)
    }

    pub async fn record_usage_and_consume(
        &self,
        key_id: &str,
        usage: TokenUsage,
        path: &str,
        model: Option<&str>,
        upstream_status: i64,
        request_id: Option<&str>,
    ) -> Result<ApiKeyRecord, AppError> {
        let usage = usage.normalize();
        let now = now_ts();
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE api_keys
            SET consumed_tokens = consumed_tokens + ?, updated_at = ?
            WHERE id = ? AND status = 'active'
            "#,
        )
        .bind(usage.total_tokens)
        .bind(now)
        .bind(key_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO usage_events (
                api_key_id, request_id, path, model, input_tokens, output_tokens,
                reasoning_tokens, cached_tokens, total_tokens, upstream_status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(key_id)
        .bind(request_id)
        .bind(path)
        .bind(model)
        .bind(usage.input_tokens)
        .bind(usage.output_tokens)
        .bind(usage.reasoning_tokens)
        .bind(usage.cached_tokens)
        .bind(usage.total_tokens)
        .bind(upstream_status)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE api_keys
            SET status = 'disabled', disabled_reason = 'quota_exceeded', disabled_at = ?, updated_at = ?
            WHERE id = ? AND status = 'active' AND consumed_tokens >= token_limit
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(key_id)
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT id, name, api_key, token_limit, consumed_tokens, status, disabled_reason, notes, created_at, updated_at, disabled_at, deleted_at
            FROM api_keys
            WHERE id = ?
            "#,
        )
        .bind(key_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("key not found"))?;

        tx.commit().await?;
        Ok(row)
    }
}

fn apply_filter(qb: &mut QueryBuilder<'_, Sqlite>, filter: &ListFilter) {
    if let Some(status) = filter.status {
        qb.push(" AND status = ");
        qb.push_bind(status.as_str());
    }
    if let Some(search) = &filter.search {
        let search = search.trim();
        if !search.is_empty() {
            let wildcard = format!("%{search}%");
            qb.push(" AND (name LIKE ");
            qb.push_bind(wildcard.clone());
            qb.push(" OR api_key LIKE ");
            qb.push_bind(wildcard);
            qb.push(")");
        }
    }
}

fn now_ts() -> i64 {
    Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn key_quota_flow() {
        let pool = crate::db::connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        let repo = Repository::new(pool);

        let key = repo
            .create_key("k1", "demo", "cpa_demo", 10, Some("notes"))
            .await
            .expect("create key");
        assert_eq!(key.status, "active");

        let updated = repo
            .record_usage_and_consume(
                "k1",
                TokenUsage {
                    total_tokens: 10,
                    ..TokenUsage::default()
                },
                "/v1/models",
                Some("gpt-4.1"),
                200,
                Some("rid-1"),
            )
            .await
            .expect("consume");
        assert_eq!(updated.consumed_tokens, 10);
        assert_eq!(updated.status, "disabled");

        let events = repo
            .list_usage_events("k1", 20)
            .await
            .expect("list events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].total_tokens, 10);
    }
}
