use serde::{Deserialize, Serialize};

use crate::config;
use crate::output::RenderForHuman;

#[derive(Serialize, Deserialize)]
pub struct LogoutResult {
    pub cleared: bool,
    pub path: Option<String>,
}

impl RenderForHuman for LogoutResult {
    fn render(&self) -> impl Into<String> {
        if self.cleared {
            match &self.path {
                Some(path) => format!("Logged out. Cleared API key in \"{}\".", path),
                None => "Logged out. Cleared stored API key.".to_string(),
            }
        } else {
            "No stored API key. You are already logged out.".to_string()
        }
    }
}

/// `topk logout`
pub fn run(config: &mut config::Config) -> LogoutResult {
    let cleared = config.api_key.take().is_some();

    LogoutResult {
        cleared,
        path: config::config_path().map(|p| p.display().to_string()),
    }
}
