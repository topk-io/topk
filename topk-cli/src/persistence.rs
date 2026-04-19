use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};

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

pub fn save_toml<T: Serialize>(path: Option<PathBuf>, value: &T) -> Result<()> {
    save_toml_with(path, value, |path, content| {
        std::fs::write(path, content)?;
        Ok(())
    })
}

pub fn save_toml_with<T: Serialize>(
    path: Option<PathBuf>,
    value: &T,
    writer: impl FnOnce(&Path, &str) -> Result<()>,
) -> Result<()> {
    let path = path.ok_or_else(|| anyhow::anyhow!("could not determine config directory"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    writer(&path, &toml::to_string_pretty(value)?)
}
