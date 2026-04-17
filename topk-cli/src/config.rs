use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub api_key: Option<String>,
}

/// Returns the path to the config file
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("topk").join("config.toml"))
}

/// Loads the config file. Returns an empty config on any read or parse error.
pub fn load() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Config::default(),
    };
    toml::from_str(&content).unwrap_or_default()
}

/// Saves the config file, creating parent directories as needed.
pub fn save(config: &Config) -> Result<()> {
    let path =
        config_path().ok_or_else(|| anyhow::anyhow!("could not determine config directory"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, toml::to_string_pretty(config)?)?;
    Ok(())
}

/// Returns a masked display string, e.g. `sk_ab...xy`.
pub fn mask(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}...{}", &key[..4], &key[key.len() - 4..])
}
