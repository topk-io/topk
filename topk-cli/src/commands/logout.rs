use std::fmt;

use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Serialize, Deserialize)]
pub struct LogoutResult {
    pub cleared: bool,
}

impl fmt::Display for LogoutResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.cleared {
            f.write_str("Logged out.")
        } else {
            f.write_str("API key not set. Skipping.")
        }
    }
}

/// `topk logout`
pub fn run(config: &config::Config) -> LogoutResult {
    LogoutResult {
        cleared: config.api_key.is_some(),
    }
}
