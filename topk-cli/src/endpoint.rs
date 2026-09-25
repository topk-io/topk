use std::fmt;

use clap::builder::NonEmptyStringValueParser;

use crate::auth::OAuthConfig;
use crate::ProjectId;

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
        value_parser = NonEmptyStringValueParser::new(),
        global = true,
        hide_env_values = true,
        help_heading = "Connection options"
    )]
    pub api_key: Option<String>,

    /// Project to access with your login instead of an API key
    #[arg(
        long,
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

    #[command(flatten)]
    pub host: Host,

    #[command(flatten)]
    pub oauth: OAuthConfig,
}

impl fmt::Debug for DataEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataEndpoint")
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("project_id", &self.project_id)
            .field("region", &self.region)
            .field("host", &self.host)
            .field("oauth", &self.oauth)
            .finish()
    }
}

#[derive(clap::Args, Clone, Debug)]
pub struct ManagementEndpoint {
    #[command(flatten)]
    pub host: Host,

    #[command(flatten)]
    pub oauth: OAuthConfig,
}
