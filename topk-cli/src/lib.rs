pub mod auth;
pub mod commands;
pub mod config;
pub mod data;
pub mod endpoint;
#[cfg(feature = "import")]
pub mod import;
pub mod management;
pub mod output;
pub mod util;

use std::fmt;
use std::str::FromStr;

use anyhow::{ensure, Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ProjectId(String);

impl FromStr for ProjectId {
    type Err = Error;

    fn from_str(id: &str) -> Result<Self> {
        ensure!(
            !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "invalid project ID: {id:?}"
        );
        Ok(Self(id.to_owned()))
    }
}

impl ProjectId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ProjectId {
    type Error = Error;

    fn try_from(id: String) -> Result<Self> {
        id.parse()
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
