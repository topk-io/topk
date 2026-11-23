use std::io::{stdin, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::{ensure, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;
use serde_json::json;
use tracing::info;

use crate::host::Host;
use crate::output::{json_line, print_list, Output, Tabular};
use crate::profile::{ProfileName, Profiles};
use crate::prompt::Prompt;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Show the profile in use
    Current,
    /// Select the profile to use
    Select { profile: Option<ProfileName> },
    /// Go back to the default profile
    Clear,
    /// List profiles
    List,
}

#[derive(Serialize)]
struct Row {
    name: ProfileName,
    #[serde(flatten)]
    host: Host,
    selected: bool,
}

impl Tabular for Row {
    fn columns(&self) -> Vec<(&'static str, String)> {
        let mut name = self.name.to_string();
        if self.selected {
            name.push('*');
        }
        vec![("Name", name), ("Host", self.host.host.clone())]
    }
}

fn rows(profiles: &Profiles, active: &ProfileName) -> Result<Vec<Row>> {
    Ok(profiles
        .list()?
        .into_iter()
        .map(|(name, host)| Row {
            selected: name == *active,
            name,
            host,
        })
        .collect())
}

pub fn run(
    profiles: &Profiles,
    active: &ProfileName,
    args: &Args,
    output: Output,
    out: &mut impl Write,
) -> Result<ExitCode> {
    info!(?args, "profile");
    match &args.command {
        Command::Current => match output {
            Output::Json => json_line(out, &json!({"profile": active}))?,
            Output::Text => writeln!(out, "Profile: {active}")?,
        },
        Command::Clear => {
            profiles.select(&ProfileName::default())?;
            match output {
                Output::Json => json_line(out, &json!({"profile": ProfileName::default()}))?,
                Output::Text => writeln!(out, "Profile selection cleared")?,
            }
        }
        Command::Select { profile } => {
            let profile = match profile {
                Some(profile) => profile.clone(),
                None => {
                    ensure!(
                        stdin().is_terminal(),
                        "no profile given; run `topk profile select <PROFILE>`"
                    );
                    let rows = rows(profiles, active)?;
                    let names = rows.iter().map(|row| &row.name).collect::<Vec<_>>();
                    let selected = Prompt::select(
                        "Select profile",
                        &names,
                        rows.iter().position(|row| row.selected),
                    )?;
                    names[selected].clone()
                }
            };
            profiles.select(&profile)?;
            match output {
                Output::Json => json_line(out, &json!({"profile": profile}))?,
                Output::Text => writeln!(out, "{} Switched to profile {profile}.", "✓".green())?,
            }
        }
        Command::List => print_list(out, output, rows(profiles, active)?)?,
    }
    Ok(ExitCode::SUCCESS)
}
