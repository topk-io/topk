use std::fmt;
use std::io::{stdin, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::{ensure, Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;
use serde_json::json;

use crate::client::ManagementClient;
use crate::config::Config;
use crate::management::proto::{
    CreateProjectRequest, DeleteProjectRequest, GetProjectRequest, ListProjectsRequest, Project,
};
use crate::output::{json_line, print_list, print_one, Output, Tabular};
use crate::util::{confirm, select, timestamp};
use crate::ProjectId;

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show the selected project
    Current,
    /// Select the default project
    Select { project_id: Option<ProjectId> },
    /// Clear the saved project selection
    Clear,
    /// List projects in the current organization
    List,
    /// Get a project by ID
    Get { project_id: String },
    /// Create a project
    Create { name: String },
    /// Delete a project by ID
    Delete {
        project_id: String,
        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

impl Tabular for Project {
    fn columns(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Name", self.name.clone()),
            ("Project ID", self.project_id.clone()),
            ("Org ID", self.org_id.clone()),
            ("Created at", timestamp(self.created_at)),
        ]
    }
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.project_id)
    }
}

#[derive(Serialize)]
struct Row {
    #[serde(flatten)]
    project: Project,
    selected: bool,
}

impl Tabular for Row {
    fn columns(&self) -> Vec<(&'static str, String)> {
        let mut columns = self.project.columns();
        if self.selected {
            columns[0].1.push('*');
        }
        columns
    }
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
            let id = config
                .defaults()?
                .project_id
                .context("no project selected; run `topk project select`")?;
            match output {
                Output::Json => json_line(out, &json!({"project_id": id}))?,
                Output::Text => writeln!(out, "Default project: {id}")?,
            }
        }
        Command::Clear => {
            config.set_project_id(None)?;
            match output {
                Output::Json => json_line(out, &json!({"project_id": null}))?,
                Output::Text => writeln!(out, "Project selection cleared")?,
            }
        }
        Command::Select { project_id } => {
            let project = match project_id {
                Some(id) => client
                    .projects
                    .get_project(GetProjectRequest {
                        project_id: id.to_string(),
                    })
                    .await?
                    .into_inner()
                    .project
                    .context("management API returned no project")?,
                None => {
                    ensure!(
                        stdin().is_terminal(),
                        "no project ID given and stdin is not a terminal; run `topk project select <PROJECT_ID>`"
                    );
                    let mut projects = client
                        .projects
                        .list_projects(ListProjectsRequest {})
                        .await?
                        .into_inner()
                        .projects;
                    let current = config.defaults()?.project_id;
                    let selected = projects.iter().position(|p| {
                        current
                            .as_ref()
                            .is_some_and(|id| id.as_str() == p.project_id)
                    });
                    let Some(index) = select("Select project", &projects, selected)? else {
                        // Cancelling the picker exits with 130.
                        return Ok(ExitCode::from(130));
                    };
                    projects.remove(index)
                }
            };
            config.set_project_id(Some(project.project_id.parse()?))?;
            match output {
                Output::Json => json_line(
                    out,
                    &Row {
                        project,
                        selected: true,
                    },
                )?,
                Output::Text => writeln!(
                    out,
                    "{} {} has been selected as the default project.",
                    "✓".green(),
                    project.name
                )?,
            }
        }
        Command::List => {
            let projects = client
                .projects
                .list_projects(ListProjectsRequest {})
                .await?
                .into_inner()
                .projects;
            let current = config.defaults()?.project_id;
            print_list(
                out,
                output,
                projects.into_iter().map(|project| Row {
                    selected: current
                        .as_ref()
                        .is_some_and(|id| id.as_str() == project.project_id),
                    project,
                }),
            )?;
        }
        Command::Get { project_id } => {
            let project = client
                .projects
                .get_project(GetProjectRequest {
                    project_id: project_id.clone(),
                })
                .await?
                .into_inner()
                .project
                .context("management API returned no project")?;
            print_one(out, output, project)?;
        }
        Command::Create { name } => {
            let project = client
                .projects
                .create_project(CreateProjectRequest { name: name.clone() })
                .await?
                .into_inner()
                .project
                .context("management API returned no project")?;
            print_one(out, output, project)?;
        }
        Command::Delete { project_id, yes } => {
            if confirm(format!("Delete project {project_id:?}?"), *yes)
                .context("pass --yes to delete without confirmation")?
            {
                client
                    .projects
                    .delete_project(DeleteProjectRequest {
                        project_id: project_id.clone(),
                    })
                    .await?;
                if output == Output::Json {
                    json_line(out, &json!({"deleted": true}))?;
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
