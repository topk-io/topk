use serde::Serialize;
use topk_rs::{
    Client, Error, client::Response,
    proto::v1::control::{CreateDatasetResponse, GetDatasetResponse, ListDatasetsResponse},
};

use crate::output::{table, RenderForHuman};
use crate::util::confirm;

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

#[derive(Serialize, serde::Deserialize)]
pub struct Dataset {
    name: String,
}

impl From<topk_rs::proto::v1::control::Dataset> for Dataset {
    fn from(dataset: topk_rs::proto::v1::control::Dataset) -> Self {
        Self {
            name: dataset.name,
        }
    }
}

#[derive(Serialize, serde::Deserialize)]
pub struct ListDatasetsResult {
    datasets: Vec<Dataset>,
}

impl From<Response<ListDatasetsResponse>> for ListDatasetsResult {
    fn from(resp: Response<ListDatasetsResponse>) -> Self {
        Self { datasets: resp.into_inner().datasets.into_iter().map(|d| d.into()).collect() }
    }
}

impl RenderForHuman for ListDatasetsResult {
    fn render(&self) -> String {
        if self.datasets.is_empty() {
            "No datasets found.".to_string()
        } else {
            table(
                vec!["NAME"],
                self.datasets.iter().map(|d| vec![d.name.clone()]).collect(),
            )
        }
    }
}

#[derive(Serialize, serde::Deserialize)]
pub struct GetDatasetResult {
    name: String,
    org_id: String,
    project_id: String,
    region: String,
}

impl TryFrom<Response<GetDatasetResponse>> for GetDatasetResult {
    type Error = Error;

    fn try_from(resp: Response<GetDatasetResponse>) -> Result<Self, Error> {
        let dataset = resp.into_inner().dataset.ok_or(Error::InvalidProto)?;
        Ok(Self { name: dataset.name, org_id: dataset.org_id, project_id: dataset.project_id, region: dataset.region })
    }
}

impl RenderForHuman for GetDatasetResult {
    fn render(&self) -> String {
        table(
            vec!["NAME"],
            vec![vec![self.name.clone()]],
        )
    }
}

#[derive(Serialize, serde::Deserialize)]
pub struct CreateDatasetResult {
    dataset: Dataset,
}

impl TryFrom<Response<CreateDatasetResponse>> for CreateDatasetResult {
    type Error = Error;

    fn try_from(resp: Response<CreateDatasetResponse>) -> Result<Self, Error> {
        let dataset = resp.into_inner().dataset.ok_or(Error::InvalidProto)?.into();
        Ok(Self { dataset })
    }
}

impl RenderForHuman for CreateDatasetResult {
    fn render(&self) -> String {
        format!("Dataset '{}' created.", self.dataset.name)
    }
}

#[derive(Serialize, serde::Deserialize)]
pub struct DeleteDatasetResult {
    deleted: bool,
    skipped: Option<bool>,
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

pub async fn list(client: &Client) -> Result<ListDatasetsResult, Error> {
    Ok(client.datasets().list().await?.into())
}

pub async fn get(client: &Client, name: &str) -> Result<GetDatasetResult, Error> {
    client.datasets().get(name).await?.try_into()
}

pub async fn create(client: &Client, name: &str) -> Result<CreateDatasetResult, Error> {
    client.datasets().create(name).await?.try_into()
}

pub async fn delete(client: &Client, name: &str, yes: bool) -> Result<DeleteDatasetResult, Error> {
    if !yes && !confirm(&format!("Delete dataset '{}'? [y/N] ", name))? {
        return Ok(DeleteDatasetResult { deleted: false, skipped: Some(true) });
    }

    let _ = client.datasets().delete(name).await?;

    Ok(DeleteDatasetResult { deleted: true, skipped: None })
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use uuid::Uuid;
    use super::{CreateDatasetResult, ListDatasetsResult};

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    fn unique_name() -> String {
        format!("topk-cli-{}", Uuid::new_v4().simple())
    }

    fn create_dataset() -> CreateDatasetResult {
        let name = unique_name();
        let out = cmd()
            .args(["--json", "dataset", "create", "--dataset", &name])
            .output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn list_datasets() -> ListDatasetsResult {
        let out = cmd().args(["--json", "dataset", "list"]).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn delete_dataset(name: &str) {
        cmd().args(["dataset", "delete", "--dataset", name, "-y"]).output().unwrap();
    }

    #[test]
    fn list() {
        let dataset = create_dataset();

        let result = list_datasets();
        let names: Vec<&str> = result.datasets.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&dataset.dataset.name.as_str()), "created dataset not in list: {:?}", names);

        delete_dataset(&dataset.dataset.name);
    }

    #[test]
    fn create() {
        let dataset = create_dataset();
        assert!(!dataset.dataset.name.is_empty());
        delete_dataset(&dataset.dataset.name);
    }

    #[test]
    fn get() {
        let dataset = create_dataset();

        let out = cmd()
            .args(["--json", "dataset", "get", "--dataset", &dataset.dataset.name])
            .output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(parsed["dataset"]["name"].as_str().unwrap(), dataset.dataset.name);

        delete_dataset(&dataset.dataset.name);
    }

    #[test]
    fn delete() {
        let dataset = create_dataset();

        let out = cmd()
            .args(["--json", "dataset", "delete", "--dataset", &dataset.dataset.name, "-y"])
            .output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(parsed["name"].as_str().unwrap(), dataset.dataset.name);
    }

    #[test]
    fn delete_aborted_when_wrong_name_entered() {
        let dataset = create_dataset();

        let out = cmd()
            .args(["--json", "dataset", "delete", "--dataset", &dataset.dataset.name])
            .write_stdin("wrong-name\n")
            .output().unwrap();
        assert!(out.status.success());
        let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(parsed["aborted"].as_bool().unwrap(), true);

        delete_dataset(&dataset.dataset.name);
    }
}
