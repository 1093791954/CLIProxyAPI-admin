use chrono::{DateTime, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyStatus {
    Active,
    Disabled,
    Deleted,
}

impl ApiKeyStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Deleted => "deleted",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "active" => Some(Self::Active),
            "disabled" => Some(Self::Disabled),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKeyRecord {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub token_limit: i64,
    pub consumed_tokens: i64,
    pub status: String,
    pub disabled_reason: Option<String>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub disabled_at: Option<i64>,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageEventRecord {
    pub id: i64,
    pub api_key_id: String,
    pub request_id: Option<String>,
    pub path: String,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub cached_tokens: i64,
    pub total_tokens: i64,
    pub upstream_status: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub cached_tokens: i64,
    pub total_tokens: i64,
}

impl TokenUsage {
    pub fn normalize(mut self) -> Self {
        if self.total_tokens <= 0 {
            self.total_tokens = self.input_tokens + self.output_tokens + self.reasoning_tokens;
        }
        if self.total_tokens <= 0 {
            self.total_tokens =
                self.input_tokens + self.output_tokens + self.reasoning_tokens + self.cached_tokens;
        }
        if self.total_tokens < 0 {
            self.total_tokens = 0;
        }
        self
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    pub token_limit: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PatchKeyRequest {
    pub name: Option<String>,
    pub token_limit: Option<i64>,
    pub notes: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListKeysQuery {
    pub status: Option<String>,
    pub search: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ListKeysResponse {
    pub items: Vec<ApiKeyDto>,
    pub page: u32,
    pub page_size: u32,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyDto {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub token_limit: i64,
    pub consumed_tokens: i64,
    pub remaining_tokens: i64,
    pub status: String,
    pub disabled_reason: Option<String>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub disabled_at: Option<i64>,
}

impl ApiKeyDto {
    pub fn status_class(&self) -> &'static str {
        match self.status.as_str() {
            "active" => "status-active",
            "disabled" => "status-disabled",
            _ => "status-muted",
        }
    }

    pub fn consumed_percent(&self) -> i64 {
        if self.token_limit <= 0 {
            return 0;
        }
        let pct = (self.consumed_tokens.saturating_mul(100)) / self.token_limit;
        pct.clamp(0, 100)
    }

    pub fn created_at_text(&self) -> String {
        format_ts(self.created_at)
    }

    pub fn updated_at_text(&self) -> String {
        format_ts(self.updated_at)
    }
}

impl From<ApiKeyRecord> for ApiKeyDto {
    fn from(value: ApiKeyRecord) -> Self {
        let remaining_tokens = (value.token_limit - value.consumed_tokens).max(0);
        Self {
            id: value.id,
            name: value.name,
            api_key: value.api_key,
            token_limit: value.token_limit,
            consumed_tokens: value.consumed_tokens,
            remaining_tokens,
            status: value.status,
            disabled_reason: value.disabled_reason,
            notes: value.notes,
            created_at: value.created_at,
            updated_at: value.updated_at,
            disabled_at: value.disabled_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UsageEventDto {
    pub id: i64,
    pub request_id: Option<String>,
    pub path: String,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub cached_tokens: i64,
    pub total_tokens: i64,
    pub upstream_status: i64,
    pub created_at: i64,
    pub created_at_text: String,
}

impl From<UsageEventRecord> for UsageEventDto {
    fn from(value: UsageEventRecord) -> Self {
        Self {
            id: value.id,
            request_id: value.request_id,
            path: value.path,
            model: value.model,
            input_tokens: value.input_tokens,
            output_tokens: value.output_tokens,
            reasoning_tokens: value.reasoning_tokens,
            cached_tokens: value.cached_tokens,
            total_tokens: value.total_tokens,
            upstream_status: value.upstream_status,
            created_at: value.created_at,
            created_at_text: format_ts(value.created_at),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateKeyForm {
    pub name: String,
    pub token_limit: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateKeyForm {
    pub name: String,
    pub token_limit: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListFilter {
    pub status: Option<ApiKeyStatus>,
    pub search: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

impl ListFilter {
    pub fn from_query(query: &ListKeysQuery) -> Self {
        let status = query.status.as_deref().and_then(ApiKeyStatus::parse);
        let search = query.search.as_ref().map(|s| s.trim().to_string());
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
        Self {
            status,
            search,
            page,
            page_size,
        }
    }
}

fn format_ts(ts: i64) -> String {
    let dt_utc: DateTime<Utc> = match Utc.timestamp_opt(ts, 0).single() {
        Some(dt) => dt,
        None => return "-".to_string(),
    };
    let dt_local: DateTime<Local> = dt_utc.with_timezone(&Local);
    dt_local.format("%Y-%m-%d %H:%M:%S").to_string()
}
