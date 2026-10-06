use std::io::Write;
use std::process::ExitCode;

use anyhow::{ensure, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;
use serde_json::json;

use crate::client::ManagementClient;
use crate::config::Config;
use crate::management::proto::ListRegionsRequest;
use crate::output::{json_line, print_list, Output, Selected, Tabular};
use crate::util::select;
use crate::Region;

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Select the default region
    Select { region: Option<Region> },
    /// Clear the saved region selection
    Unselect,
    /// List available regions
    List,
}

#[derive(Serialize)]
struct RegionRow {
    name: Region,
}

impl Tabular for RegionRow {
    fn columns(&self) -> Vec<(&'static str, String)> {
        vec![("Name", self.name.to_string())]
    }
}

pub async fn run(
    config: Config,
    args: &Args,
    output: Output,
    out: &mut impl Write,
) -> Result<ExitCode> {
    match &args.command {
        None => {
            let region = config.defaults()?.region;
            if output == Output::Json {
                json_line(out, &json!({"region": region}))?;
            } else if let Some(region) = region {
                writeln!(out, "Default region: {region}")?;
            } else {
                writeln!(out, "No default region selected")?;
            }
        }
        Some(Command::Unselect) => {
            config.set_region(None).await?;
            if output == Output::Json {
                json_line(out, &json!({"region": null}))?;
            } else {
                writeln!(out, "Region selection cleared")?;
            }
        }
        Some(command) => {
            let mut client = ManagementClient::new(config.clone())?;
            let mut regions = client
                .regions
                .list_regions(ListRegionsRequest {})
                .await?
                .into_inner()
                .regions
                .into_iter()
                .map(|region| region.parse())
                .collect::<Result<Vec<Region>>>()?;
            let current = config.defaults()?.region;
            match command {
                Command::List => print_list(
                    out,
                    output,
                    regions.into_iter().map(|name| Selected {
                        selected: current.as_ref() == Some(&name),
                        item: RegionRow { name },
                    }),
                )?,
                Command::Select { region } => {
                    let region = match region {
                        Some(region) => {
                            ensure!(regions.contains(region), "unknown region: {region}");
                            region.clone()
                        }
                        None => {
                            let Some(index) = select(
                                "Select region",
                                &regions.iter().map(ToString::to_string).collect::<Vec<_>>(),
                                regions.iter().position(|r| Some(r) == current.as_ref()),
                            )?
                            else {
                                return Ok(ExitCode::SUCCESS);
                            };
                            regions.remove(index)
                        }
                    };
                    config.set_region(Some(region.clone())).await?;
                    if output == Output::Json {
                        json_line(
                            out,
                            &Selected {
                                item: RegionRow { name: region },
                                selected: true,
                            },
                        )?;
                    } else {
                        writeln!(
                            out,
                            "{} {region} has been selected as the default region.",
                            "✓".green()
                        )?;
                    }
                }
                Command::Unselect => unreachable!("handled above"),
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
