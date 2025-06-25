use pyo3::prelude::*;

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Text(),
    Integer(),
    Float(),
    Boolean(),
    F32Vector { dimension: u32 },
    U8Vector { dimension: u32 },
    BinaryVector { dimension: u32 },
    F32SparseVector(),
    U8SparseVector(),
    Bytes(),
}

impl Into<topk_rs::proto::v1::control::field_type::DataType> for DataType {
    fn into(self) -> topk_rs::proto::v1::control::field_type::DataType {
        match self {
            DataType::Integer() => topk_rs::proto::v1::control::field_type::DataType::integer(),
            DataType::Float() => topk_rs::proto::v1::control::field_type::DataType::float(),
            DataType::Text() => topk_rs::proto::v1::control::field_type::DataType::text(),
            DataType::Boolean() => topk_rs::proto::v1::control::field_type::DataType::bool(),
            DataType::F32Vector { dimension } => {
                topk_rs::proto::v1::control::field_type::DataType::f32_vector(dimension)
            }
            DataType::U8Vector { dimension } => {
                topk_rs::proto::v1::control::field_type::DataType::u8_vector(dimension)
            }
            DataType::BinaryVector { dimension } => {
                topk_rs::proto::v1::control::field_type::DataType::binary_vector(dimension)
            }
            DataType::F32SparseVector() => {
                topk_rs::proto::v1::control::field_type::DataType::f32_sparse_vector()
            }
            DataType::U8SparseVector() => {
                topk_rs::proto::v1::control::field_type::DataType::u8_sparse_vector()
            }
            DataType::Bytes() => topk_rs::proto::v1::control::field_type::DataType::bytes(),
        }
    }
}

impl Into<topk_rs::proto::v1::control::FieldType> for DataType {
    fn into(self) -> topk_rs::proto::v1::control::FieldType {
        topk_rs::proto::v1::control::FieldType {
            data_type: Some(self.into()),
        }
    }
}

impl From<topk_rs::proto::v1::control::field_type::DataType> for DataType {
    fn from(proto: topk_rs::proto::v1::control::field_type::DataType) -> Self {
        match proto {
            topk_rs::proto::v1::control::field_type::DataType::Integer(_) => DataType::Integer(),
            topk_rs::proto::v1::control::field_type::DataType::Float(_) => DataType::Float(),
            topk_rs::proto::v1::control::field_type::DataType::Text(_) => DataType::Text(),
            topk_rs::proto::v1::control::field_type::DataType::Boolean(_) => DataType::Boolean(),
            topk_rs::proto::v1::control::field_type::DataType::F32Vector(vector) => {
                DataType::F32Vector {
                    dimension: vector.dimension,
                }
            }
            topk_rs::proto::v1::control::field_type::DataType::U8Vector(vector) => {
                DataType::U8Vector {
                    dimension: vector.dimension,
                }
            }
            topk_rs::proto::v1::control::field_type::DataType::BinaryVector(vector) => {
                DataType::BinaryVector {
                    dimension: vector.dimension,
                }
            }
            topk_rs::proto::v1::control::field_type::DataType::F32SparseVector(_) => {
                DataType::F32SparseVector()
            }
            topk_rs::proto::v1::control::field_type::DataType::U8SparseVector(_) => {
                DataType::U8SparseVector()
            }
            topk_rs::proto::v1::control::field_type::DataType::Bytes(_) => DataType::Bytes(),
        }
    }
}
