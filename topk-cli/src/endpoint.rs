use std::fmt;
use std::str::FromStr;

use anyhow::{ensure, Error, Result};

use crate::auth::Config;

#[derive(clap::Args, Clone, Debug)]
pub struct Host {
    /// API domain
    #[arg(
        long,
        env = "TOPK_HOST",
        default_value = "topk.io",
        global = true,
        hide = true
    )]
    pub host: String,

    /// Connect over HTTPS (default: true; --https false for a plaintext endpoint)
    #[arg(
        long,
        env = "TOPK_HTTPS",
        default_value = "true",
        num_args = 0..=1,
        default_missing_value = "true",
        global = true,
        hide = true
    )]
    pub https: bool,
}

#[derive(clap::Args, Clone)]
pub struct DataEndpoint {
    /// TopK API key
    #[arg(
        long,
        env = "TOPK_API_KEY",
        global = true,
        hide_env_values = true,
        help_heading = "Connection options"
    )]
    pub api_key: Option<String>,

    /// Project to access with your login instead of an API key
    #[arg(long, global = true, help_heading = "Connection options")]
    pub project_id: Option<ProjectId>,

    /// Region to read and write; list available regions at https://docs.topk.io/regions
    #[arg(
        long,
        env = "TOPK_REGION",
        global = true,
        help_heading = "Connection options"
    )]
    pub region: Option<String>,

    #[command(flatten)]
    pub host: Host,

    #[command(flatten)]
    pub auth: Config,
}

impl fmt::Debug for DataEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataEndpoint")
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("project_id", &self.project_id)
            .field("region", &self.region)
            .field("host", &self.host)
            .field("auth", &self.auth)
            .finish()
    }
}

/// A project's identifier, as accepted on the command line.
///
/// Project IDs name cache files directly, so anything that could leave a directory is
/// rejected when parsed rather than at the point of use.
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(clap::Args, Clone, Debug)]
pub struct ManagementEndpoint {
    #[command(flatten)]
    pub host: Host,

    #[command(flatten)]
    pub auth: Config,
}

/// The management API of the same host, which mints a data endpoint's project tokens.
impl From<&DataEndpoint> for ManagementEndpoint {
    fn from(data: &DataEndpoint) -> Self {
        Self {
            host: data.host.clone(),
            auth: data.auth.clone(),
        }
    }
}
