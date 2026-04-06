use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    org_id: String,
    #[pyo3(get)]
    project_id: String,
    #[pyo3(get)]
    region: String,
    #[pyo3(get)]
    created_at: String,
}

#[pymethods]
impl Dataset {
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

impl From<topk_rs::proto::v1::control::Dataset> for Dataset {
    fn from(dataset: topk_rs::proto::v1::control::Dataset) -> Self {
        Self {
            name: dataset.name,
            org_id: dataset.org_id,
            project_id: dataset.project_id,
            region: dataset.region,
            created_at: dataset.created_at,
        }
    }
}
