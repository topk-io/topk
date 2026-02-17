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
}

#[pymethods]
impl Dataset {
    #[new]
    pub fn new(
        name: String,
        org_id: String,
        project_id: String,
        region: String,
    ) -> Self {
        Self {
            name,
            org_id,
            project_id,
            region,
        }
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    pub fn __eq__(&self, other: &Dataset) -> bool {
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
        }
    }
}
