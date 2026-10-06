use std::io::Write;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde_json::json;

use crate::client::ManagementClient;
use crate::config::Config;
use crate::management::proto::{
    CreateProjectRequest, DeleteProjectRequest, GetProjectRequest, ListProjectsRequest, Project,
};
use crate::output::{json_line, print, print_list, Output, Selected, Tabular};
use crate::util::{confirm, select, timestamp};
use crate::ProjectId;

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Select the default project
    Select { project_id: Option<ProjectId> },
    /// Clear the saved project selection
    Unselect,
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
            ("Created at", timestamp(self.created_at)),
        ]
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
            let project_id = config.defaults()?.project_id;
            if output == Output::Json {
                json_line(out, &json!({"project_id": project_id}))?;
            } else if let Some(id) = project_id {
                let project = ManagementClient::new(config.clone())?
                    .projects
                    .get_project(GetProjectRequest {
                        project_id: id.to_string(),
                    })
                    .await
                    .with_context(|| format!("loading default project {id}"))?
                    .into_inner()
                    .project
                    .context("management API returned no project")?;
                writeln!(out, "Default project: {} ({id})", project.name)?;
            } else {
                writeln!(out, "No default project selected")?;
            }
            return Ok(ExitCode::SUCCESS);
        }
        Some(Command::Unselect) => {
            config.set_project_id(None).await?;
            if output == Output::Json {
                json_line(out, &json!({"project_id": null}))?;
            } else {
                writeln!(out, "Project selection cleared")?;
            }
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }
    let mut client = ManagementClient::new(config.clone())?;
    match args.command.as_ref().expect("handled missing command") {
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
                    let mut projects = client
                        .projects
                        .list_projects(ListProjectsRequest {})
                        .await?
                        .into_inner()
                        .projects;
                    let current = config.defaults()?.project_id;
                    let labels = projects
                        .iter()
                        .map(|p| format!("{} ({})", p.name, p.project_id))
                        .collect::<Vec<_>>();
                    let Some(index) = select(
                        "Select project",
                        &labels,
                        projects.iter().position(|p| {
                            current
                                .as_ref()
                                .is_some_and(|id| id.as_str() == p.project_id)
                        }),
                    )?
                    else {
                        return Ok(ExitCode::SUCCESS);
                    };
                    projects.remove(index)
                }
            };
            let id = project.project_id.parse()?;
            config.set_project_id(Some(id)).await?;
            if output == Output::Json {
                json_line(
                    out,
                    &Selected {
                        item: project,
                        selected: true,
                    },
                )?;
            } else {
                writeln!(
                    out,
                    "{} {} has been selected as the default project.",
                    "✓".green(),
                    project.name
                )?;
            }
        }
        Command::Unselect => unreachable!("handled above"),
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
                projects.into_iter().map(|project| Selected {
                    selected: current
                        .as_ref()
                        .is_some_and(|id| id.as_str() == project.project_id),
                    item: project,
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
            print(out, output, project)?;
        }
        Command::Create { name } => {
            let project = client
                .projects
                .create_project(CreateProjectRequest { name: name.clone() })
                .await?
                .into_inner()
                .project
                .context("management API returned no project")?;
            print(out, output, project)?;
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
