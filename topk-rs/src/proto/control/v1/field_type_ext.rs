use crate::proto::control::v1::{
    field_type_list::ListValueType, field_type_matrix::MatrixValueType,
};

use super::*;

impl FieldType {
    pub fn text() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::Text(FieldTypeText {})),
        }
    }

    pub fn integer() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::Integer(FieldTypeInteger {})),
        }
    }

    pub fn float() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::Float(FieldTypeFloat {})),
        }
    }

    pub fn boolean() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::Boolean(FieldTypeBoolean {})),
        }
    }

    pub fn bytes() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::Bytes(FieldTypeBytes {})),
        }
    }

    pub fn list(value_type: ListValueType) -> Self {
        FieldType {
            data_type: Some(field_type::DataType::List(FieldTypeList {
                value_type: value_type.into(),
            })),
        }
    }

    pub fn f32_vector(dimension: u32) -> Self {
        FieldType {
            data_type: Some(field_type::DataType::F32Vector(FieldTypeF32Vector {
                dimension,
            })),
        }
    }

    pub fn f16_vector(dimension: u32) -> Self {
        FieldType {
            data_type: Some(field_type::DataType::F16Vector(FieldTypeF16Vector {
                dimension,
            })),
        }
    }

    pub fn f8_vector(dimension: u32) -> Self {
        FieldType {
            data_type: Some(field_type::DataType::F8Vector(FieldTypeF8Vector {
                dimension,
            })),
        }
    }

    pub fn u8_vector(dimension: u32) -> Self {
        FieldType {
            data_type: Some(field_type::DataType::U8Vector(FieldTypeU8Vector {
                dimension,
            })),
        }
    }

    pub fn i8_vector(dimension: u32) -> Self {
        FieldType {
            data_type: Some(field_type::DataType::I8Vector(FieldTypeI8Vector {
                dimension,
            })),
        }
    }

    pub fn binary_vector(dimension: u32) -> Self {
        FieldType {
            data_type: Some(field_type::DataType::BinaryVector(FieldTypeBinaryVector {
                dimension,
            })),
        }
    }

    pub fn f32_sparse_vector() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::F32SparseVector(
                FieldTypeF32SparseVector {},
            )),
        }
    }

    pub fn f16_sparse_vector() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::F16SparseVector(
                FieldTypeF16SparseVector {},
            )),
        }
    }

    pub fn f8_sparse_vector() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::F8SparseVector(
                FieldTypeF8SparseVector {},
            )),
        }
    }

    pub fn u8_sparse_vector() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::U8SparseVector(
                FieldTypeU8SparseVector {},
            )),
        }
    }

    pub fn i8_sparse_vector() -> Self {
        FieldType {
            data_type: Some(field_type::DataType::I8SparseVector(
                FieldTypeI8SparseVector {},
            )),
        }
    }

    pub fn matrix(num_cols: u32, value_type: MatrixValueType) -> Self {
        FieldType {
            data_type: Some(field_type::DataType::Matrix(FieldTypeMatrix {
                dimension: num_cols,
                value_type: value_type.into(),
            })),
        }
    }

    pub fn r#struct(fields: impl IntoIterator<Item = (impl Into<String>, FieldSpec)>) -> Self {
        FieldType {
            data_type: Some(field_type::DataType::r#struct(fields)),
        }
    }
}
