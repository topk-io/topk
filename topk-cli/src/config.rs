use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tempfile::NamedTempFile;
use topk_rs::Error;

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
    load_toml_or_default(config_path(), |_, _| {})
}

/// Saves the config file, creating parent directories as needed.
pub fn save(config: &Config) -> Result<(), Error> {
    save_toml_with(config_path(), config, write_config_file)
}

pub fn set_api_key(api_key: String) -> Result<(), Error> {
    let mut config = load();
    config.api_key = Some(api_key);
    save(&config)
}

pub fn clear() -> Result<(), Error> {
    save(&Config::default())
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
    std::fs::write(path, content).map_err(Error::IoError)?;
    Ok(())
}

pub fn load_toml_or_default<T>(
    path: Option<PathBuf>,
    on_parse_error: impl FnOnce(&Path, &toml::de::Error),
) -> T
where
    T: DeserializeOwned + Default,
{
    let path = match path {
        Some(p) => p,
        None => return T::default(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return T::default(),
    };
    match toml::from_str(&content) {
        Ok(value) => value,
        Err(err) => {
            on_parse_error(&path, &err);
            T::default()
        }
    }
}

pub fn save_toml<T: Serialize>(path: Option<PathBuf>, value: &T) -> Result<(), Error> {
    save_toml_with(path, value, |path, content| {
        use std::io::Write;

        let parent = path.parent().ok_or_else(|| {
            Error::Input(anyhow::anyhow!("could not determine parent directory"))
        })?;
        let mut tmp = NamedTempFile::new_in(parent).map_err(Error::IoError)?;
        tmp.write_all(content.as_bytes()).map_err(Error::IoError)?;
        tmp.flush().map_err(Error::IoError)?;
        tmp.as_file().sync_all().map_err(Error::IoError)?;
        tmp.persist(path).map_err(|e| Error::IoError(e.error))?;
        Ok(())
    })
}

pub fn save_toml_with<T: Serialize>(
    path: Option<PathBuf>,
    value: &T,
    writer: impl FnOnce(&Path, &str) -> Result<(), Error>,
) -> Result<(), Error> {
    let path =
        path.ok_or_else(|| Error::Input(anyhow::anyhow!("could not determine config directory")))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    writer(
        &path,
        &toml::to_string_pretty(value).map_err(|e| Error::MalformedResponse(e.to_string()))?,
    )
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
