use anyhow::{Context, Result};
use clap::builder::NonEmptyStringValueParser;

#[derive(clap::Args, Clone, Debug)]
pub struct DataEndpoint {
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

impl DataEndpoint {
    pub fn region(&self) -> Result<&str> {
        self.region.as_deref().context(
            "--region is required (or set TOPK_REGION). \
             List available regions with `topk region list` at https://docs.topk.io/regions",
        )
    }
}
