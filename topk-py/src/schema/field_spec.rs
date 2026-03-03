use crate::error::RustError;
use crate::schema::data_type::DataType;
use crate::schema::field_index::FieldIndex;
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

impl TryFrom<topk_rs::proto::v1::control::FieldSpec> for FieldSpec {
    type Error = RustError;

    fn try_from(proto: topk_rs::proto::v1::control::FieldSpec) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            data_type: proto
                .data_type
                .and_then(|d| d.data_type)
                .ok_or(topk_rs::Error::InvalidProto)?
                .try_into()?,
            required: proto.required,
            index: proto
                .index
                .map(|i| i.try_into())
                .transpose()?,
        })
    }
}
