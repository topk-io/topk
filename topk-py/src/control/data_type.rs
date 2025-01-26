use pyo3::prelude::*;

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Text(),
    Integer(),
    Float(),
    Boolean(),
    FloatVector { dimension: u32 },
    ByteVector { dimension: u32 },
    Bytes(),
}

impl Into<topk_protos::v1::control::field_type::DataType> for DataType {
    fn into(self) -> topk_protos::v1::control::field_type::DataType {
        match self {
            DataType::Integer() => topk_protos::v1::control::field_type::DataType::integer(),
            DataType::Float() => topk_protos::v1::control::field_type::DataType::float(),
            DataType::Text() => topk_protos::v1::control::field_type::DataType::text(),
            DataType::Boolean() => topk_protos::v1::control::field_type::DataType::bool(),
            DataType::FloatVector { dimension } => {
                topk_protos::v1::control::field_type::DataType::float_vector(dimension)
            }
            DataType::ByteVector { dimension } => {
                topk_protos::v1::control::field_type::DataType::byte_vector(dimension)
            }
            DataType::Bytes() => topk_protos::v1::control::field_type::DataType::bytes(),
        }
    }
}

impl Into<topk_protos::v1::control::FieldType> for DataType {
    fn into(self) -> topk_protos::v1::control::FieldType {
        topk_protos::v1::control::FieldType {
            data_type: Some(self.into()),
        }
    }
}

impl From<topk_protos::v1::control::field_type::DataType> for DataType {
    fn from(proto: topk_protos::v1::control::field_type::DataType) -> Self {
        match proto {
            topk_protos::v1::control::field_type::DataType::Integer(_) => DataType::Integer(),
            topk_protos::v1::control::field_type::DataType::Float(_) => DataType::Float(),
            topk_protos::v1::control::field_type::DataType::Text(_) => DataType::Text(),
            topk_protos::v1::control::field_type::DataType::Boolean(_) => DataType::Boolean(),
            topk_protos::v1::control::field_type::DataType::FloatVector(vector) => {
                DataType::FloatVector {
                    dimension: vector.dimension,
                }
            }
            topk_protos::v1::control::field_type::DataType::ByteVector(vector) => {
                DataType::ByteVector {
                    dimension: vector.dimension,
                }
            }
            topk_protos::v1::control::field_type::DataType::Bytes(_) => DataType::Bytes(),
        }
    }
}
