use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::auth::CredentialsStore;

/// Shared settings persisted in config.toml.
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<CredentialsStore>,
    #[serde(flatten)]
    pub extra: toml::Table,
}

/// The CLI configuration directory.
pub fn dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("topk"))
}
