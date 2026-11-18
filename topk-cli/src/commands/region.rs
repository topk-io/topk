use std::io::{stdin, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::{ensure, Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;
use serde_json::json;
use tracing::info;

use crate::client::ManagementClient;
use crate::config::Config;
use crate::output::{json_line, print_list, Output, Tabular};
use crate::scope::pick_region;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
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

pub async fn run(
    config: Config,
    args: &Args,
    output: Output,
    out: &mut impl Write,
) -> Result<ExitCode> {
    info!(?args, "region");
    let client = ManagementClient::new(config.clone())?;
    match args.command.as_ref().unwrap_or(&Command::List) {
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
                        client.regions().list().await?.contains(region),
                        "unknown region {region:?}; list regions with `topk region list`"
                    );
                    region.clone()
                }
                None => {
                    ensure!(
                        stdin().is_terminal(),
                        "no region given; run `topk region select <REGION>`"
                    );
                    pick_region(&client, &config).await?
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
                client.regions().list().await?.into_iter().map(|name| Row {
                    selected: current.as_ref() == Some(&name),
                    name,
                }),
            )?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
