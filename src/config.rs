use std::env;

use crate::error::AppError;

const MIN_ADMIN_USERNAME_LEN: usize = 6;
const MIN_ADMIN_PASSWORD_LEN: usize = 16;
const WEAK_PASSWORD_PATTERNS: &[&str] = &["password", "123456", "admin", "qwerty", "letmein"];

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub upstream_base_url: String,
    pub upstream_bearer_key: String,
    pub public_notice_file: Option<String>,
    pub admin_username: String,
    pub admin_password: String,
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
        let admin_username = env::var("ADMIN_USERNAME").map_err(|_| {
            AppError::config("missing env ADMIN_USERNAME. set it in .env or process environment")
        })?;
        let admin_password = env::var("ADMIN_PASSWORD").map_err(|_| {
            AppError::config("missing env ADMIN_PASSWORD. set it in .env or process environment")
        })?;
        let public_notice_file = env::var("PUBLIC_NOTICE_FILE")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        let upstream_bearer_key = upstream_bearer_key.trim().to_string();
        let admin_username = admin_username.trim().to_string();
        let admin_password = admin_password.trim().to_string();

        if upstream_bearer_key.is_empty() {
            return Err(AppError::config("UPSTREAM_BEARER_KEY cannot be empty"));
        }
        if admin_username.is_empty() {
            return Err(AppError::config("ADMIN_USERNAME cannot be empty"));
        }
        if admin_password.is_empty() {
            return Err(AppError::config("ADMIN_PASSWORD cannot be empty"));
        }

        validate_admin_credentials(&admin_username, &admin_password)?;

        Ok(Self {
            bind_addr,
            database_url,
            upstream_base_url: upstream_base_url.trim_end_matches('/').to_string(),
            upstream_bearer_key,
            public_notice_file,
            admin_username,
            admin_password,
        })
    }
}

fn validate_admin_credentials(username: &str, password: &str) -> Result<(), AppError> {
    validate_admin_username(username)?;
    validate_admin_password(username, password)?;
    Ok(())
}

fn validate_admin_username(username: &str) -> Result<(), AppError> {
    if username.len() < MIN_ADMIN_USERNAME_LEN {
        return Err(AppError::config(format!(
            "ADMIN_USERNAME is too short: must be at least {MIN_ADMIN_USERNAME_LEN} characters"
        )));
    }
    if !username.is_ascii() || !username.bytes().all(|b| (b'!'..=b'~').contains(&b)) {
        return Err(AppError::config(
            "ADMIN_USERNAME must use visible ASCII characters without spaces",
        ));
    }
    Ok(())
}

fn validate_admin_password(username: &str, password: &str) -> Result<(), AppError> {
    if password.len() < MIN_ADMIN_PASSWORD_LEN {
        return Err(AppError::config(format!(
            "ADMIN_PASSWORD is too short: must be at least {MIN_ADMIN_PASSWORD_LEN} characters"
        )));
    }
    if password.chars().any(char::is_whitespace) {
        return Err(AppError::config(
            "ADMIN_PASSWORD cannot contain whitespace characters",
        ));
    }

    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| c.is_ascii_punctuation());

    if !has_upper || !has_lower || !has_digit || !has_special {
        return Err(AppError::config(
            "ADMIN_PASSWORD must include uppercase, lowercase, digit and special character",
        ));
    }

    let password_lower = password.to_ascii_lowercase();
    let username_lower = username.to_ascii_lowercase();
    if password_lower.contains(&username_lower) {
        return Err(AppError::config(
            "ADMIN_PASSWORD cannot contain ADMIN_USERNAME",
        ));
    }
    if WEAK_PASSWORD_PATTERNS
        .iter()
        .any(|pattern| password_lower.contains(pattern))
    {
        return Err(AppError::config(
            "ADMIN_PASSWORD is too weak: contains common weak password pattern",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_admin_credentials, validate_admin_password, validate_admin_username};

    #[test]
    fn rejects_short_username() {
        let err = validate_admin_username("admin").expect_err("expected short username to fail");
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn rejects_username_with_space() {
        let err = validate_admin_username("admin user")
            .expect_err("expected username with space to fail");
        assert!(err.to_string().contains("visible ASCII"));
    }

    #[test]
    fn rejects_short_password() {
        let err = validate_admin_password("adminops", "Aa1!short")
            .expect_err("expected short password to fail");
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn rejects_missing_password_category() {
        let err = validate_admin_password("adminops", "alllowercase123456!")
            .expect_err("expected missing uppercase to fail");
        assert!(err.to_string().contains("must include"));
    }

    #[test]
    fn rejects_password_containing_username() {
        let err = validate_admin_password("AdminOps", "xxadminopsAA11!!yy")
            .expect_err("expected password containing username to fail");
        assert!(err.to_string().contains("cannot contain"));
    }

    #[test]
    fn rejects_common_weak_pattern() {
        let err = validate_admin_password("adminops", "passwordAA11!!zzzz")
            .expect_err("expected common weak pattern to fail");
        assert!(err.to_string().contains("weak"));
    }

    #[test]
    fn accepts_strong_credentials() {
        validate_admin_credentials("adminops", "S7rong!Token#Keeper2026")
            .expect("expected strong credentials to pass");
    }
}
