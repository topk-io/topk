use std::fmt;

use clap::builder::NonEmptyStringValueParser;

use crate::{ProjectId, Region};

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

    /// Project to access with your login instead of an API key
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
        global = true,
        help_heading = "Connection options"
    )]
    pub region: Option<Region>,
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
