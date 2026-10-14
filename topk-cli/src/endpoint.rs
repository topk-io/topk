use std::fmt;

use anyhow::{bail, Context, Result};
use clap::builder::NonEmptyStringValueParser;

use crate::config::Config;
use crate::ProjectId;

#[derive(clap::Args, Clone)]
pub struct DataEndpoint {
    /// TopK API key
    #[arg(
        long,
        env = "TOPK_API_KEY",
        value_parser = NonEmptyStringValueParser::new(),
        global = true,
        hide_env_values = true,
        help_heading = "Connection options"
    )]
    pub api_key: Option<String>,

    /// Project to access with your login instead of an API key; defaults to the currently
    /// selected project
    #[arg(
        long,
        env = "TOPK_PROJECT_ID",
        conflicts_with = "api_key",
        global = true,
        help_heading = "Connection options"
    )]
    pub project_id: Option<ProjectId>,

    /// Region to read and write; list available regions at https://docs.topk.io/regions
    #[arg(
        long,
        env = "TOPK_REGION",
        value_parser = NonEmptyStringValueParser::new(),
        global = true,
        help_heading = "Connection options"
    )]
    pub region: Option<String>,
}

pub enum Credentials {
    ApiKey(String),
    Project(ProjectId),
}

impl DataEndpoint {
    pub fn region(&self) -> Result<&str> {
        self.region.as_deref().context(
            "--region is required (or set TOPK_REGION). \
             List available regions with `topk region list` at https://docs.topk.io/regions",
        )
    }

    /// An API key or `--project-id` wins over the project saved by `topk project select`.
    pub fn credentials(&self, config: &Config) -> Result<Credentials> {
        match (&self.api_key, &self.project_id) {
            // Clap rejects the combinations; this guards direct construction.
            (Some(_), Some(_)) => bail!("--project-id cannot be combined with an API key"),
            (Some(api_key), None) => Ok(Credentials::ApiKey(api_key.clone())),
            (None, Some(project_id)) => Ok(Credentials::Project(project_id.clone())),
            (None, None) => match config.defaults()?.project_id {
                Some(project_id) => Ok(Credentials::Project(project_id)),
                None => bail!(
                    "--project-id (or TOPK_PROJECT_ID) or --api-key (or TOPK_API_KEY) is required, \
                     or select a default project with `topk project select`"
                ),
            },
        }
    }
}

impl fmt::Debug for DataEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataEndpoint")
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("project_id", &self.project_id)
            .field("region", &self.region)
            .finish()
    }
}
