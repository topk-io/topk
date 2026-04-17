use comfy_table::{
    presets, Attribute, Cell, Color, ColumnConstraint, ContentArrangement, Table, Width,
};
use serde::{Deserialize, Serialize};
use terminal_size::{terminal_size, Width as TermWidth};
use topk_rs::{
    proto::v1::control::{CreateDatasetResponse, GetDatasetResponse, ListDatasetsResponse},
    Error,
};

use crate::datasets::DatasetsClient;
use crate::output::{Output, RenderForHuman};
use crate::util::format_timestamp;

/// `topk dataset`
#[derive(Debug, clap::Subcommand)]
pub enum DatasetAction {
    /// List all datasets
    List,
    /// Get a dataset
    Get {
        /// Dataset name
        #[arg(value_name = "DATASET")]
        dataset: String,
    },
    /// Create a dataset
    Create {
        /// Dataset name
        #[arg(value_name = "DATASET")]
        dataset: String,
        /// Region to create the dataset in. List available regions at https://docs.topk.io/regions
        #[arg(long, required = true)]
        region: String,
    },
    /// Delete a dataset
    Delete {
        /// Dataset name
        #[arg(value_name = "DATASET")]
        dataset: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    pub region: String,
    // RFC3339 formatted timestamp
    pub(crate) created_at: String,
}

impl From<topk_rs::proto::v1::control::Dataset> for Dataset {
    fn from(dataset: topk_rs::proto::v1::control::Dataset) -> Self {
        Self {
            name: dataset.name,
            region: dataset.region,
            created_at: dataset.created_at,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListDatasetsResult {
    pub datasets: Vec<Dataset>,
}

impl From<ListDatasetsResponse> for ListDatasetsResult {
    fn from(resp: ListDatasetsResponse) -> Self {
        Self {
            datasets: resp.datasets.into_iter().map(|d| d.into()).collect(),
        }
    }
}

impl RenderForHuman for ListDatasetsResult {
    fn render(&self) -> impl Into<String> {
        if self.datasets.is_empty() {
            return "No datasets found.".to_string();
        }

        let term_width = terminal_size().map(|(TermWidth(w), _)| w).unwrap_or(80);

        let mut table = Table::new();
        table
            .load_preset(presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(term_width)
            .set_header(
                ["NAME", "REGION", "CREATED"]
                    .iter()
                    .map(|h| Cell::new(h).add_attribute(Attribute::Bold).fg(Color::Cyan)),
            )
            .set_constraints([
                ColumnConstraint::LowerBoundary(Width::Fixed(10)),
                ColumnConstraint::ContentWidth,
                ColumnConstraint::ContentWidth,
            ]);

        for d in &self.datasets {
            table.add_row([
                Cell::new(&d.name),
                Cell::new(&d.region),
                Cell::new(format_timestamp(&d.created_at).unwrap_or_default())
                    .add_attribute(Attribute::Dim),
            ]);
        }

        table.to_string()
    }
}

#[derive(Serialize, Deserialize)]
pub struct GetDatasetResult {
    pub(crate) dataset: Dataset,
}

impl TryFrom<GetDatasetResponse> for GetDatasetResult {
    type Error = Error;

    fn try_from(resp: GetDatasetResponse) -> Result<Self, Error> {
        Ok(Self {
            dataset: resp
                .dataset
                .ok_or_else(|| Error::MalformedResponse("dataset missing from get response".to_string()))?
                .into(),
        })
    }
}

impl RenderForHuman for GetDatasetResult {
    fn render(&self) -> impl Into<String> {
        format!(
            "Name:    {}\nRegion:  {}\nCreated: {}",
            self.dataset.name,
            self.dataset.region,
            format_timestamp(&self.dataset.created_at).unwrap_or_default()
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreateDatasetResult {
    pub(crate) dataset: Dataset,
}

impl TryFrom<CreateDatasetResponse> for CreateDatasetResult {
    type Error = Error;

    fn try_from(resp: CreateDatasetResponse) -> Result<Self, Error> {
        Ok(Self {
            dataset: resp
                .dataset
                .ok_or_else(|| Error::MalformedResponse("dataset missing from create response".to_string()))?
                .into(),
        })
    }
}

impl RenderForHuman for CreateDatasetResult {
    fn render(&self) -> impl Into<String> {
        format!("Dataset '{}' created.", self.dataset.name)
    }
}

#[derive(Serialize, Deserialize)]
pub struct DeleteDatasetResult {
    pub deleted: bool,
}

impl RenderForHuman for DeleteDatasetResult {
    fn render(&self) -> impl Into<String> {
        if self.deleted {
            "Dataset deleted.".to_string()
        } else {
            "Deletion skipped.".to_string()
        }
    }
}

/// `topk dataset list`
pub async fn list<C: DatasetsClient + ?Sized>(client: &mut C) -> Result<ListDatasetsResult, Error> {
    Ok(client.list().await?.into())
}

/// `topk dataset get`
pub async fn get<C: DatasetsClient + ?Sized>(
    client: &mut C,
    name: &str,
) -> Result<GetDatasetResult, Error> {
    client.get(name).await?.try_into()
}

/// `topk dataset create`
pub async fn create<C: DatasetsClient + ?Sized>(
    client: &mut C,
    name: &str,
    region: &str,
) -> Result<CreateDatasetResult, Error> {
    client.create(name, region).await?.try_into()
}

/// `topk dataset delete`
pub async fn delete<C: DatasetsClient + ?Sized>(
    client: &mut C,
    name: &str,
    yes: bool,
    output: &Output,
) -> Result<DeleteDatasetResult, Error> {
    if !yes && !output.confirm(&format!("Delete dataset '{}'? ", name))? {
        return Ok(DeleteDatasetResult { deleted: false });
    }

    client.delete(name).await?;

    Ok(DeleteDatasetResult { deleted: true })
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
            .args(["dataset", "create", "--region", &ctx.region, &name])
            .output()
            .unwrap();

        let out = cmd()
            .args(["-o", "json", "dataset", "list"])
            .output()
            .unwrap();
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
            .args(["-o", "json", "dataset", "create", "--region", &ctx.region, &name])
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
            .args(["dataset", "create", "--region", &ctx.region, &name])
            .output()
            .unwrap();

        let out = cmd()
            .args(["-o", "json", "dataset", "get", &name])
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
            .args(["dataset", "create", "--region", &ctx.region, &name])
            .output()
            .unwrap();

        let out = cmd()
            .args(["-o", "json", "dataset", "delete", &name, "-y"])
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
            .args(["dataset", "create", "--region", &ctx.region, &name])
            .output()
            .unwrap();

        let out = cmd()
            .args(["-o", "json", "dataset", "delete", &name])
            .write_stdin("wrong-name\n")
            .output()
            .unwrap();
        assert!(out.status.success());
        let result: DeleteDatasetResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(!result.deleted);
    }
}
