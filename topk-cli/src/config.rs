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

/// Returns a masked display string, e.g. `****mnop`.
pub fn mask(key: &str) -> String {
    let chars: Vec<_> = key.chars().collect();
    if chars.len() <= 4 {
        return "*".repeat(chars.len());
    }
    let suffix: String = chars.iter().rev().take(4).copied().collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}{}", "*".repeat(chars.len() - 4), suffix)
}

#[cfg(test)]
mod tests {
    use super::mask;

    #[test]
    fn mask_handles_ascii_keys() {
        assert_eq!(mask("sk_abcdefghijklmnop"), "***************mnop");
        assert_eq!(mask("short"), "*hort");
    }

    #[test]
    fn mask_handles_multibyte_keys() {
        assert_eq!(mask("密钥🔒安全令牌XYZ"), "******牌XYZ");
        assert_eq!(mask("🔒短"), "**");
    }
}
