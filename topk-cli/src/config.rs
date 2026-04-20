use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::persistence;

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
    persistence::load_toml_or_default(config_path(), |_, _| {})
}

/// Saves the config file, creating parent directories as needed.
pub fn save(config: &Config) -> Result<()> {
    persistence::save_toml_with(config_path(), config, write_config_file)
}

#[cfg(unix)]
fn write_config_file(path: &std::path::Path, content: &str) -> Result<()> {
    use std::fs::{OpenOptions, Permissions};
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    file.set_permissions(Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn write_config_file(path: &std::path::Path, content: &str) -> Result<()> {
    std::fs::write(path, content)?;
    Ok(())
}

/// Returns a masked display string, e.g. `****mnop`.
pub fn mask(key: &str) -> String {
    let chars: Vec<_> = key.chars().collect();
    if chars.len() <= 4 {
        return "*".repeat(chars.len());
    }
    let suffix: String = chars
        .iter()
        .rev()
        .take(4)
        .copied()
        .collect::<Vec<_>>()
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

    #[cfg(unix)]
    #[test]
    fn write_config_file_uses_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("config.toml");

        super::write_config_file(&path, "api_key = 'secret'").expect("write config");

        let mode = std::fs::metadata(&path)
            .expect("stat config")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn config_with_none_api_key_serializes_and_roundtrips() {
        let config = super::Config { api_key: None };

        let toml = toml::to_string(&config).expect("serialize config");
        let restored: super::Config = toml::from_str(&toml).expect("deserialize config");

        assert!(restored.api_key.is_none());
    }
}
