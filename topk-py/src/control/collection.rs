use crate::control::field_spec::FieldSpec;
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass]
#[derive(Debug, Clone)]
pub struct Collection {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    org_id: u64,
    #[pyo3(get)]
    project_id: u32,
    #[pyo3(get)]
    schema: HashMap<String, FieldSpec>,
}

#[pymethods]
impl Collection {
    #[new]
    pub fn new(
        name: String,
        org_id: u64,
        project_id: u32,
        schema: HashMap<String, FieldSpec>,
    ) -> Self {
        Self {
            name,
            org_id,
            project_id,
            schema,
        }
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    pub fn __eq__(&self, other: &Collection) -> bool {
        self.name == other.name
    }
}

impl Into<topk_protos::v1::control::Index> for Collection {
    fn into(self) -> topk_protos::v1::control::Index {
        let schema = topk_protos::v1::control::index_schema::IndexSchema::new(
            self.schema
                .into_iter()
                .map(|(name, field)| (name, field.into()))
                .collect(),
        );

        topk_protos::v1::control::Index::new(self.name, self.org_id, self.project_id, schema)
    }
}

impl From<topk_protos::v1::control::Index> for Collection {
    fn from(index: topk_protos::v1::control::Index) -> Self {
        let mut schema = HashMap::new();
        for (name, field) in index.schema {
            schema.insert(name, field.into());
        }

        Self {
            name: index.name,
            org_id: index.org_id,
            project_id: index.project_id,
            schema,
        }
    }
}
