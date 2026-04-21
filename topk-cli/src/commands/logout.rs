use serde::{Deserialize, Serialize};

use crate::config;
use crate::output::RenderForHuman;

#[derive(Serialize, Deserialize)]
pub struct LogoutResult {
    pub cleared: bool,
}

impl RenderForHuman for LogoutResult {
    fn render(&self) -> impl Into<String> {
        if self.cleared {
            "Logged out.".to_string()
        } else {
            "API key not set. Skipping.".to_string()
        }
    }
}

/// `topk logout`
pub fn run(config: &config::Config) -> LogoutResult {
    LogoutResult {
        cleared: config.api_key.is_some(),
    }
}
