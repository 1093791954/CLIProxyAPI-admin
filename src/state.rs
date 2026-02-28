use std::sync::Arc;

use reqwest::Client;

use crate::{config::AppConfig, repository::Repository};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub repo: Repository,
    pub http_client: Client,
}
