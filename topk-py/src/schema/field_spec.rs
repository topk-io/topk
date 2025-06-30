use crate::schema::data_type::DataType;
use crate::schema::field_index::{FieldIndex, KeywordIndexType, VectorDistanceMetric};
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSpec {
    data_type: DataType,
    required: bool,
    index: Option<FieldIndex>,
}

#[pymethods]
impl FieldSpec {
    #[new]
    pub fn new(data_type: DataType) -> Self {
        Self {
            data_type,
            required: false,
            index: None,
        }
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    pub fn __eq__(&self, other: &FieldSpec) -> bool {
        self == other
    }

    pub fn required(&self) -> Self {
        Self {
            required: true,
            ..self.clone()
        }
    }

    pub fn optional(&self) -> Self {
        Self {
            required: false,
            ..self.clone()
        }
    }

    pub fn keyword_index(&self) -> Self {
        self.index(FieldIndex::KeywordIndex {
            index_type: KeywordIndexType::Text,
        })
    }

    pub fn vector_index(&self, metric: VectorDistanceMetric) -> Self {
        self.index(FieldIndex::VectorIndex { metric })
    }

    fn index(&self, index: FieldIndex) -> Self {
        Self {
            index: Some(index),
            ..self.clone()
        }
    }
}

impl Into<topk_rs::proto::v1::control::FieldSpec> for FieldSpec {
    fn into(self) -> topk_rs::proto::v1::control::FieldSpec {
        topk_rs::proto::v1::control::FieldSpec {
            data_type: Some(self.data_type.into()),
            required: self.required,
            index: self.index.map(|i| i.into()),
        }
    }
}

impl From<topk_rs::proto::v1::control::FieldSpec> for FieldSpec {
    fn from(proto: topk_rs::proto::v1::control::FieldSpec) -> Self {
        Self {
            data_type: {
                let a = proto
                    .data_type
                    .map(|d| d.data_type)
                    .flatten()
                    .map(|d| d.into());

                if a.is_none() {
                    panic!("supposedly invalid proto: {:?}", proto);
                }

                a.expect("data_type is required")
            },
            required: proto.required,
            index: proto.index.map(|i| i.into()),
        }
    }
}
