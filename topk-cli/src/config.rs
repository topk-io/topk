use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use topk_rs::Error;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub api_key: Option<String>,
}

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("topk").join("config.toml"))
}

pub fn load() -> Config {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn set_api_key(api_key: String) -> Result<(), Error> {
    save(&Config {
        api_key: Some(api_key),
    })
}

pub fn clear() -> Result<(), Error> {
    save(&Config::default())
}

fn save(config: &Config) -> Result<(), Error> {
    let path = config_path()
        .ok_or_else(|| Error::Input(anyhow::anyhow!("could not determine config directory")))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content =
        toml::to_string_pretty(config).map_err(|e| Error::MalformedResponse(e.to_string()))?;
    write_config_file(&path, &content)
}

#[cfg(unix)]
fn write_config_file(path: &std::path::Path, content: &str) -> Result<(), Error> {
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
fn write_config_file(path: &std::path::Path, content: &str) -> Result<(), Error> {
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
