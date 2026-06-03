use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct Partition {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub created_at: String,
}

impl From<topk_rs::proto::v1::data::Partition> for Partition {
    fn from(partition: topk_rs::proto::v1::data::Partition) -> Self {
        Self {
            name: partition.name,
            created_at: partition.created_at,
        }
    }
}
