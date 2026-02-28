use rand::{Rng, distributions::Alphanumeric};
use ulid::Ulid;

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
