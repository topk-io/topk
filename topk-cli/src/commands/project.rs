use std::io::Write;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::json;

use crate::endpoint::ManagementEndpoint;
use crate::management::proto::{
    CreateProjectRequest, DeleteProjectRequest, GetProjectRequest, ListProjectsRequest, Project,
};
use crate::management::Client;
use crate::output::{json_line, print, Tabular};
use crate::util::{confirm, timestamp};

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub endpoint: ManagementEndpoint,
}

#[derive(Subcommand)]
pub enum Command {
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

pub async fn run(
    client: &mut Client,
    args: &Args,
    json: bool,
    out: &mut impl Write,
) -> Result<ExitCode> {
    match &args.command {
        Command::List => {
            let projects = client
                .projects
                .list_projects(ListProjectsRequest {})
                .await?
                .into_inner()
                .projects;
            print(out, json, projects)?;
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
            print(out, json, [project])?;
        }
        Command::Create { name } => {
            let project = client
                .projects
                .create_project(CreateProjectRequest { name: name.clone() })
                .await?
                .into_inner()
                .project
                .context("management API returned no project")?;
            print(out, json, [project])?;
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
                if json {
                    json_line(out, &json!({"deleted": true}))?;
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
