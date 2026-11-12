use std::io::{stdin, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::{ensure, Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;
use serde_json::json;

use crate::client::ManagementClient;
use crate::config::Config;
use crate::management::proto::ListRegionsRequest;
use crate::output::{json_line, print_list, Output, Tabular};
use crate::scope::pick_region;

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show the selected region
    Current,
    /// Select the default region
    Select { region: Option<String> },
    /// Clear the saved region selection
    Clear,
    /// List available regions
    List,
}

#[derive(Serialize)]
struct Row {
    name: String,
    selected: bool,
}

impl Tabular for Row {
    fn columns(&self) -> Vec<(&'static str, String)> {
        let mut name = self.name.clone();
        if self.selected {
            name.push('*');
        }
        vec![("Name", name)]
    }
}

async fn list(client: &mut ManagementClient) -> Result<Vec<String>> {
    Ok(client
        .regions
        .list_regions(ListRegionsRequest {})
        .await?
        .into_inner()
        .regions)
}

pub async fn run(
    config: Config,
    args: &Args,
    output: Output,
    out: &mut impl Write,
) -> Result<ExitCode> {
    let mut client = ManagementClient::new(config.clone())?;
    match &args.command {
        Command::Current => {
            let region = config
                .defaults()?
                .region
                .context("no region selected; run `topk region select`")?;
            match output {
                Output::Json => json_line(out, &json!({"region": region}))?,
                Output::Text => writeln!(out, "Default region: {region}")?,
            }
        }
        Command::Clear => {
            config.set_region(None)?;
            match output {
                Output::Json => json_line(out, &json!({"region": null}))?,
                Output::Text => writeln!(out, "Region selection cleared")?,
            }
        }
        Command::Select { region } => {
            let region = match region {
                Some(region) => {
                    ensure!(
                        list(&mut client).await?.contains(region),
                        "unknown region {region:?}; list regions with `topk region list`"
                    );
                    region.clone()
                }
                None => {
                    ensure!(
                        stdin().is_terminal(),
                        "no region given and stdin is not a terminal; run `topk region select <REGION>`"
                    );
                    pick_region(&mut client, &config).await?
                }
            };
            config.set_region(Some(region.clone()))?;
            match output {
                Output::Json => json_line(
                    out,
                    &Row {
                        name: region,
                        selected: true,
                    },
                )?,
                Output::Text => writeln!(
                    out,
                    "{} {region} has been selected as the default region.",
                    "✓".green()
                )?,
            }
        }
        Command::List => {
            let current = config.defaults()?.region;
            print_list(
                out,
                output,
                list(&mut client).await?.into_iter().map(|name| Row {
                    selected: current.as_ref() == Some(&name),
                    name,
                }),
            )?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
