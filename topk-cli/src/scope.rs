use std::io::{stdin, IsTerminal};

use anyhow::{ensure, Result};
use clap::builder::NonEmptyStringValueParser;
use serde::{Deserialize, Serialize};

use crate::client::ManagementClient;
use crate::config::Config;
use crate::management::proto::{ListProjectsRequest, ListRegionsRequest, Project};
use crate::util::select;
use crate::ProjectId;

#[derive(clap::Args, Clone, Debug)]
pub struct ScopeArgs {
    /// Project to access; defaults to the currently selected project
    #[arg(long, env = "TOPK_PROJECT_ID", help_heading = "Connection options")]
    project_id: Option<ProjectId>,

    /// Region to read and write; defaults to the currently selected region
    #[arg(
        long,
        env = "TOPK_REGION",
        value_parser = NonEmptyStringValueParser::new(),
        help_heading = "Connection options"
    )]
    region: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scope {
    pub project_id: ProjectId,
    pub region: String,
}

impl ScopeArgs {
    pub async fn resolve(&self, config: &Config) -> Result<Scope> {
        Ok(Scope {
            project_id: self.project_id(config).await?,
            region: self.region(config).await?,
        })
    }

    pub async fn project_id(&self, config: &Config) -> Result<ProjectId> {
        if let Some(id) = &self.project_id {
            return Ok(id.clone());
        }
        if let Some(id) = config.defaults()?.project_id {
            return Ok(id);
        }
        ensure!(
            stdin().is_terminal(),
            "--project-id (or TOPK_PROJECT_ID environment variable) is required, \
             or select a default project with `topk project select`"
        );
        let mut client = ManagementClient::new(config.clone())?;
        let project = pick_project(&mut client, config).await?;
        eprintln!("Run `topk project select` to skip this prompt.");
        project.project_id.parse()
    }

    async fn region(&self, config: &Config) -> Result<String> {
        if let Some(region) = &self.region {
            return Ok(region.clone());
        }
        if let Some(region) = config.defaults()?.region {
            return Ok(region);
        }
        ensure!(
            stdin().is_terminal(),
            "--region (or TOPK_REGION environment variable) is required, \
             or select a default region with `topk region select`"
        );
        let mut client = ManagementClient::new(config.clone())?;
        let region = pick_region(&mut client, config).await?;
        eprintln!("Run `topk region select` to skip this prompt.");
        Ok(region)
    }
}

pub async fn pick_project(client: &mut ManagementClient, config: &Config) -> Result<Project> {
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
    Ok(projects.remove(select("Select project", &projects, selected)?))
}

pub async fn pick_region(client: &mut ManagementClient, config: &Config) -> Result<String> {
    let mut regions = client
        .regions
        .list_regions(ListRegionsRequest {})
        .await?
        .into_inner()
        .regions;
    let current = config.defaults()?.region;
    let selected = regions
        .iter()
        .position(|name| current.as_ref() == Some(name));
    Ok(regions.remove(select("Select region", &regions, selected)?))
}
