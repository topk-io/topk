use napi_derive::napi;

/// Represents a dataset in the TopK service.
#[napi(object)]
pub struct Dataset {
    /// Name of the dataset
    pub name: String,
    /// Dataset description
    pub description: Option<String>,
    /// Organization ID that owns the dataset
    pub org_id: String,
    /// Project ID that contains the dataset
    pub project_id: String,
    /// Region where the dataset is stored
    pub region: String,
    /// RFC 3339 timestamp when the dataset was created
    pub created_at: String,
}

impl From<topk_rs::proto::v1::control::Dataset> for Dataset {
    fn from(dataset: topk_rs::proto::v1::control::Dataset) -> Self {
        Self {
            name: dataset.name,
            description: dataset.description,
            org_id: dataset.org_id,
            project_id: dataset.project_id,
            region: dataset.region,
            created_at: dataset.created_at,
        }
    }
}
