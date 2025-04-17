use super::value::Value;
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub fields: HashMap<String, Value>,
}

#[pymethods]
impl Document {
    fn __repr__(&self) -> String {
        format!("{}", self)
    }
}

impl std::fmt::Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "doc(")?;
        let mut fields = self.fields.iter().collect::<Vec<(&String, &Value)>>();
        fields.sort_by(|a, b| a.0.cmp(&b.0));

        for (i, (k, v)) in fields.iter().enumerate() {
            if i < fields.len() - 1 {
                write!(f, "{}={:?}, ", k, v)?;
            } else {
                write!(f, "{}={:?}", k, v)?;
            }
        }
        write!(f, ")")
    }
}

impl Into<topk_protos::v1::data::Document> for Document {
    fn into(self) -> topk_protos::v1::data::Document {
        topk_protos::v1::data::Document::from(self.fields.into_iter().map(|(k, v)| (k, v.into())))
    }
}

impl TryFrom<topk_protos::v1::data::Document> for Document {
    type Error = anyhow::Error;

    fn try_from(proto: topk_protos::v1::data::Document) -> Result<Self, Self::Error> {
        let mut fields = HashMap::new();

        for (k, v) in proto.fields {
            fields.insert(k, v.try_into()?);
        }

        Ok(Self { fields })
    }
}
