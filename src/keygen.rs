use rand::{Rng, distributions::Alphanumeric};
use ulid::Ulid;

use crate::error::AppError;

pub const API_KEY_MIN_LEN: usize = 8;
pub const API_KEY_MAX_LEN: usize = 128;

pub fn new_key_id() -> String {
    Ulid::new().to_string().to_lowercase()
}

pub fn new_api_key() -> String {
    let suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(28)
        .map(char::from)
        .collect();
    format!("cpa_{}", suffix.to_lowercase())
}

pub fn resolve_api_key(input: Option<&str>) -> Result<String, AppError> {
    let candidate = input.map(str::trim).unwrap_or_default();
    if candidate.is_empty() {
        return Ok(new_api_key());
    }

    let len = candidate.chars().count();
    if !(API_KEY_MIN_LEN..=API_KEY_MAX_LEN).contains(&len) {
        return Err(AppError::bad_request(format!(
            "api_key length must be between {API_KEY_MIN_LEN} and {API_KEY_MAX_LEN} characters"
        )));
    }

    Ok(candidate.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_generates_key() {
        let key = resolve_api_key(None).expect("auto key");
        assert!(key.starts_with("cpa_"));
    }

    #[test]
    fn blank_input_generates_key() {
        let key = resolve_api_key(Some("   ")).expect("auto key");
        assert!(key.starts_with("cpa_"));
    }

    #[test]
    fn custom_key_is_accepted() {
        let key = resolve_api_key(Some("sk-aabbccdd")).expect("custom key");
        assert_eq!(key, "sk-aabbccdd");
    }

    #[test]
    fn short_key_is_rejected() {
        let err = resolve_api_key(Some("short")).expect_err("should reject short key");
        assert!(err.to_string().contains("api_key length"));
    }
}
