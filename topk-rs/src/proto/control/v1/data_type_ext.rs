use crate::proto::control::v1::field_type_matrix::MatrixValueType;

use super::*;

impl field_type::DataType {
    pub fn text() -> Self {
        field_type::DataType::Text(FieldTypeText {})
    }

    pub fn integer() -> Self {
        field_type::DataType::Integer(FieldTypeInteger {})
    }

    pub fn float() -> Self {
        field_type::DataType::Float(FieldTypeFloat {})
    }

    pub fn bool() -> Self {
        field_type::DataType::Boolean(FieldTypeBoolean {})
    }

    pub fn f32_vector(dimension: u32) -> Self {
        field_type::DataType::F32Vector(FieldTypeF32Vector { dimension })
    }

    pub fn f16_vector(dimension: u32) -> Self {
        field_type::DataType::F16Vector(FieldTypeF16Vector { dimension })
    }

    pub fn f8_vector(dimension: u32) -> Self {
        field_type::DataType::F8Vector(FieldTypeF8Vector { dimension })
    }

    pub fn u8_vector(dimension: u32) -> Self {
        field_type::DataType::U8Vector(FieldTypeU8Vector { dimension })
    }

    pub fn i8_vector(dimension: u32) -> Self {
        field_type::DataType::I8Vector(FieldTypeI8Vector { dimension })
    }

    pub fn matrix(dimension: u32, value_type: MatrixValueType) -> Self {
        field_type::DataType::Matrix(FieldTypeMatrix {
            dimension,
            value_type: value_type.into(),
        })
    }

    pub fn binary_vector(dimension: u32) -> Self {
        field_type::DataType::BinaryVector(FieldTypeBinaryVector { dimension })
    }

    pub fn f32_sparse_vector() -> Self {
        field_type::DataType::F32SparseVector(FieldTypeF32SparseVector {})
    }

    pub fn u8_sparse_vector() -> Self {
        field_type::DataType::U8SparseVector(FieldTypeU8SparseVector {})
    }

    pub fn bytes() -> Self {
        field_type::DataType::Bytes(FieldTypeBytes {})
    }
}
