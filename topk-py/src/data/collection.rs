use crate::schema::field_spec::FieldSpec;
use pyo3::prelude::*;
use std::collections::HashMap;
use topk_rs::proto::v1::control::FieldSpec as FieldSpecPb;

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct Collection {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    org_id: String,
    #[pyo3(get)]
    project_id: String,
    #[pyo3(get)]
    region: String,
    #[pyo3(get)]
    schema: HashMap<String, FieldSpec>,
    #[pyo3(get)]
    created_at: String,
}

#[pymethods]
impl Collection {
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    pub fn __eq__(&self, other: &Collection) -> bool {
        self == other
    }
}

impl Into<topk_rs::proto::v1::control::Collection> for Collection {
    fn into(self) -> topk_rs::proto::v1::control::Collection {
        let schema = self
            .schema
            .into_iter()
            .map(|(name, field)| (name, field.into()))
            .collect::<HashMap<String, FieldSpecPb>>();

        topk_rs::proto::v1::control::Collection {
            name: self.name,
            org_id: self.org_id.to_string(),
            project_id: self.project_id.to_string(),
            schema,
            region: self.region.to_string(),
            created_at: self.created_at.to_string(),
        }
    }
}

impl From<topk_rs::proto::v1::control::Collection> for Collection {
    fn from(collection: topk_rs::proto::v1::control::Collection) -> Self {
        Self {
            name: collection.name,
            org_id: collection.org_id,
            project_id: collection.project_id,
            region: collection.region,
            schema: collection
                .schema
                .into_iter()
                .map(|(name, field)| (name, field.into()))
                .collect(),
            created_at: collection.created_at,
        }
    }
}
