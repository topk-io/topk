use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use topk_rs::{
    client::Response,
    proto::v1::control::{CreateDatasetResponse, GetDatasetResponse, ListDatasetsResponse},
    Client, Error,
};

use crate::output::{table, Output, RenderForHuman};

/// `topk dataset`
#[derive(Debug, clap::Subcommand)]
pub enum DatasetAction {
    /// List all datasets
    List,
    /// Get a dataset
    Get {
        /// Dataset name
        #[arg(long, value_name = "DATASET_NAME")]
        dataset: String,
    },
    /// Create a dataset
    Create {
        /// Dataset name
        #[arg(long, value_name = "DATASET_NAME")]
        dataset: String,
    },
    /// Delete a dataset
    Delete {
        /// Dataset name
        #[arg(long, value_name = "DATASET_NAME")]
        dataset: String,
        /// Skip confirmation prompt
        #[arg(short = 'y')]
        yes: bool,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Dataset {
    pub(crate) name: String,
    /// RFC3339 formatted timestamp
    pub(crate) created_at: String,
}

impl From<topk_rs::proto::v1::control::Dataset> for Dataset {
    fn from(dataset: topk_rs::proto::v1::control::Dataset) -> Self {
        Self {
            name: dataset.name,
            created_at: dataset.created_at,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListDatasetsResult {
    pub(crate) datasets: Vec<Dataset>,
}

impl From<Response<ListDatasetsResponse>> for ListDatasetsResult {
    fn from(resp: Response<ListDatasetsResponse>) -> Self {
        Self {
            datasets: resp
                .into_inner()
                .datasets
                .into_iter()
                .map(|d| d.into())
                .collect(),
        }
    }
}

impl RenderForHuman for ListDatasetsResult {
    fn render(&self) -> String {
        if self.datasets.is_empty() {
            "No datasets found.".to_string()
        } else {
            table(
                vec!["Name", "Created"],
                self.datasets
                    .iter()
                    .map(|d| {
                        vec![
                            d.name.clone(),
                            d.created_at
                                .parse::<DateTime<Utc>>()
                                .unwrap_or_default()
                                .with_timezone(&Local)
                                .format("%b %-d, %Y %-H:%M")
                                .to_string(),
                        ]
                    })
                    .collect(),
            )
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GetDatasetResult {
    pub(crate) dataset: Dataset,
}

impl TryFrom<Response<GetDatasetResponse>> for GetDatasetResult {
    type Error = Error;

    fn try_from(resp: Response<GetDatasetResponse>) -> Result<Self, Error> {
        Ok(Self {
            dataset: resp.into_inner().dataset.ok_or(Error::InvalidProto)?.into(),
        })
    }
}

impl RenderForHuman for GetDatasetResult {
    fn render(&self) -> String {
        table(vec!["Name"], vec![vec![self.dataset.name.clone()]])
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreateDatasetResult {
    pub(crate) dataset: Dataset,
}

impl TryFrom<Response<CreateDatasetResponse>> for CreateDatasetResult {
    type Error = Error;

    fn try_from(resp: Response<CreateDatasetResponse>) -> Result<Self, Error> {
        Ok(Self {
            dataset: resp.into_inner().dataset.ok_or(Error::InvalidProto)?.into(),
        })
    }
}

impl RenderForHuman for CreateDatasetResult {
    fn render(&self) -> String {
        format!("Dataset '{}' created.", self.dataset.name)
    }
}

#[derive(Serialize, Deserialize)]
pub struct DeleteDatasetResult {
    pub(crate) deleted: bool,
    pub(crate) skipped: Option<bool>,
}

impl RenderForHuman for DeleteDatasetResult {
    fn render(&self) -> String {
        if self.skipped == Some(true) {
            "Deletion skipped.".to_string()
        } else {
            "Dataset deleted.".to_string()
        }
    }
}

/// `topk dataset list`
pub async fn list(client: &Client) -> Result<ListDatasetsResult, Error> {
    Ok(client.datasets().list().await?.into())
}

/// `topk dataset get`
pub async fn get(client: &Client, name: &str) -> Result<GetDatasetResult, Error> {
    client.datasets().get(name).await?.try_into()
}

/// `topk dataset create`
pub async fn create(client: &Client, name: &str) -> Result<CreateDatasetResult, Error> {
    client.datasets().create(name).await?.try_into()
}

/// `topk dataset delete`
pub async fn delete(
    client: &Client,
    name: &str,
    yes: bool,
    output: &Output,
) -> Result<DeleteDatasetResult, Error> {
    if !yes && !output.confirm(&format!("Delete dataset '{}'? [y/N] ", name))? {
        return Ok(DeleteDatasetResult {
            deleted: false,
            skipped: Some(true),
        });
    }

    let _ = client.datasets().delete(name).await?;

    Ok(DeleteDatasetResult {
        deleted: true,
        skipped: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{CreateDatasetResult, DeleteDatasetResult, GetDatasetResult, ListDatasetsResult};
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use test_context::test_context;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn list(ctx: &mut CliTestContext) {
        let name = ctx.wrap("test");
        cmd()
            .args(["dataset", "create", "--dataset", &name])
            .output()
            .unwrap();

        let out = cmd().args(["--json", "dataset", "list"]).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: ListDatasetsResult = serde_json::from_slice(&out.stdout).unwrap();
        let names: Vec<&str> = result.datasets.iter().map(|d| d.name.as_str()).collect();
        assert!(
            names.contains(&name.as_str()),
            "created dataset not in list: {:?}",
            names
        );
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn create(ctx: &mut CliTestContext) {
        let name = ctx.wrap("test");
        let out = cmd()
            .args(["--json", "dataset", "create", "--dataset", &name])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: CreateDatasetResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.dataset.name, name);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn get(ctx: &mut CliTestContext) {
        let name = ctx.wrap("test");
        cmd()
            .args(["dataset", "create", "--dataset", &name])
            .output()
            .unwrap();

        let out = cmd()
            .args(["--json", "dataset", "get", "--dataset", &name])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: GetDatasetResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.dataset.name, name);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn delete(ctx: &mut CliTestContext) {
        let name = ctx.wrap("test");
        cmd()
            .args(["dataset", "create", "--dataset", &name])
            .output()
            .unwrap();

        let out = cmd()
            .args(["--json", "dataset", "delete", "--dataset", &name, "-y"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: DeleteDatasetResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(result.deleted);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn delete_aborted(ctx: &mut CliTestContext) {
        let name = ctx.wrap("test");
        cmd()
            .args(["dataset", "create", "--dataset", &name])
            .output()
            .unwrap();

        let out = cmd()
            .args(["--json", "dataset", "delete", "--dataset", &name])
            .write_stdin("wrong-name\n")
            .output()
            .unwrap();
        assert!(out.status.success());
        let result: DeleteDatasetResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.skipped, Some(true));
    }
}
