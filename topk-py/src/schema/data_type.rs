use pyo3::prelude::*;

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Text(),
    Integer(),
    Float(),
    Boolean(),
    Bytes(),
    F32Vector { dimension: u32 },
    U8Vector { dimension: u32 },
    BinaryVector { dimension: u32 },
    F32Matrix { dimension: u32 },
    U8Matrix { dimension: u32 },
    BinaryMatrix { dimension: u32 },
}

impl Into<topk_protos::v1::control::field_type::DataType> for DataType {
    fn into(self) -> topk_protos::v1::control::field_type::DataType {
        match self {
            DataType::Integer() => topk_protos::v1::control::field_type::DataType::integer(),
            DataType::Float() => topk_protos::v1::control::field_type::DataType::float(),
            DataType::Text() => topk_protos::v1::control::field_type::DataType::text(),
            DataType::Boolean() => topk_protos::v1::control::field_type::DataType::bool(),
            DataType::Bytes() => topk_protos::v1::control::field_type::DataType::bytes(),
            DataType::F32Vector { dimension } => {
                topk_protos::v1::control::field_type::DataType::f32_vector(dimension)
            }
            DataType::U8Vector { dimension } => {
                topk_protos::v1::control::field_type::DataType::u8_vector(dimension)
            }
            DataType::BinaryVector { dimension } => {
                topk_protos::v1::control::field_type::DataType::binary_vector(dimension)
            }
            DataType::F32Matrix { dimension } => {
                topk_protos::v1::control::field_type::DataType::f32_matrix(dimension)
            }
            DataType::U8Matrix { dimension } => {
                topk_protos::v1::control::field_type::DataType::u8_matrix(dimension)
            }
            DataType::BinaryMatrix { dimension } => {
                topk_protos::v1::control::field_type::DataType::binary_matrix(dimension)
            }
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
            topk_protos::v1::control::field_type::DataType::Bytes(_) => DataType::Bytes(),
            // Vector types
            topk_protos::v1::control::field_type::DataType::F32Vector(vector) => {
                DataType::F32Vector {
                    dimension: vector.dimension,
                }
            }
            topk_protos::v1::control::field_type::DataType::U8Vector(vector) => {
                DataType::U8Vector {
                    dimension: vector.dimension,
                }
            }
            topk_protos::v1::control::field_type::DataType::BinaryVector(vector) => {
                DataType::BinaryVector {
                    dimension: vector.dimension,
                }
            }
            // Matrix types
            topk_protos::v1::control::field_type::DataType::F32Matrix(matrix) => {
                DataType::F32Matrix {
                    dimension: matrix.dimension,
                }
            }
            topk_protos::v1::control::field_type::DataType::U8Matrix(matrix) => {
                DataType::U8Matrix {
                    dimension: matrix.dimension,
                }
            }
            topk_protos::v1::control::field_type::DataType::BinaryMatrix(matrix) => {
                DataType::BinaryMatrix {
                    dimension: matrix.dimension,
                }
            }
        }
    }
}
