use crate::schema::field_spec::FieldSpec;
use pyo3::prelude::*;
use std::collections::HashMap;
use topk_protos::v1::control::FieldSpec as FieldSpecPb;

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
}

#[pymethods]
impl Collection {
    #[new]
    pub fn new(
        name: String,
        org_id: String,
        project_id: String,
        region: String,
        schema: HashMap<String, FieldSpec>,
    ) -> Self {
        Self {
            name,
            org_id,
            project_id,
            region,
            schema,
        }
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    pub fn __eq__(&self, other: &Collection) -> bool {
        self == other
    }
}

impl Into<topk_protos::v1::control::Collection> for Collection {
    fn into(self) -> topk_protos::v1::control::Collection {
        let schema = self
            .schema
            .into_iter()
            .map(|(name, field)| (name, field.into()))
            .collect::<HashMap<String, FieldSpecPb>>();

        topk_protos::v1::control::Collection::new(
            self.name,
            self.org_id.to_string(),
            self.project_id.to_string(),
            schema,
            self.region.to_string(),
        )
    }
}

impl From<topk_protos::v1::control::Collection> for Collection {
    fn from(collection: topk_protos::v1::control::Collection) -> Self {
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
        }
    }
}
