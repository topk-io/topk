use std::io::Write;
use std::process::ExitCode;

use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;

use crate::client::ManagementClient;
use crate::config::Config;
use crate::management::proto::ListRegionsRequest;
use crate::output::{print, Output, Tabular};

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List available regions
    List,
}

#[derive(Serialize)]
struct Region {
    name: String,
}

impl Tabular for Region {
    fn columns(&self) -> Vec<(&'static str, String)> {
        vec![("Name", self.name.clone())]
    }
}

pub async fn run(
    config: Config,
    args: &Args,
    output: Output,
    out: &mut impl Write,
) -> Result<ExitCode> {
    let mut client = ManagementClient::new(config)?;
    match args.command {
        Command::List => {
            let regions = client
                .regions
                .list_regions(ListRegionsRequest {})
                .await?
                .into_inner()
                .regions;
            print(out, output, regions.into_iter().map(|name| Region { name }))?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
