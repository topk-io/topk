use std::io::Write;
use std::process::ExitCode;

use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;

use crate::endpoint::ManagementEndpoint;
use crate::management::proto::ListRegionsRequest;
use crate::management::Client;
use crate::output::{print, Tabular};

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub endpoint: ManagementEndpoint,
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
    client: &mut Client,
    args: &Args,
    json: bool,
    out: &mut impl Write,
) -> Result<ExitCode> {
    match args.command {
        Command::List => {
            let regions = client
                .regions
                .list_regions(ListRegionsRequest {})
                .await?
                .into_inner()
                .regions;
            print(out, json, regions.into_iter().map(|name| Region { name }))?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
