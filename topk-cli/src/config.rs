use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub api_key: Option<String>,
}

pub fn dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("topk"))
}

pub fn config_path() -> Option<PathBuf> {
    dir().map(|d| d.join("config.toml"))
}

pub fn load() -> Result<Config> {
    let path = match config_path() {
        Some(path) => path,
        None => return Ok(Config::default()),
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            toml::from_str(&text).with_context(|| format!("cannot parse {}", path.display()))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
    }
}

pub fn set_api_key(api_key: String) -> Result<()> {
    save(&Config {
        api_key: Some(api_key),
    })
}

pub fn clear() -> Result<()> {
    save(&Config::default())
}

fn save(config: &Config) -> Result<()> {
    let path = config_path().context("could not determine config directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    write_config_file(&path, &content)
}

#[cfg(unix)]
fn write_config_file(path: &Path, content: &str) -> Result<()> {
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
fn write_config_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
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
