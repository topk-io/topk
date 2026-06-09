use crate::proto::control::v1::{
    field_type_list::ListValueType, field_type_matrix::MatrixValueType,
};

use super::*;

impl FieldSpec {
    pub fn with_index(mut self, index: FieldIndex) -> Self {
        assert!(self.index.is_none(), "Field index is already set");
        self.index = Some(index);
        self
    }

    pub fn text(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::text()),
            required,
            index: None,
        }
    }

    pub fn integer(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::integer()),
            required,
            index: None,
        }
    }

    pub fn float(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::float()),
            required,
            index: None,
        }
    }

    pub fn boolean(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::boolean()),
            required,
            index: None,
        }
    }

    pub fn bytes(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::bytes()),
            required,
            index: None,
        }
    }

    pub fn list(required: bool, value_type: ListValueType) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::list(value_type)),
            required,
            index: None,
        }
    }

    pub fn f32_vector(dimension: u32, required: bool) -> FieldSpec {
        Self::vector_field(FieldType::f32_vector(dimension), required)
    }

    pub fn f16_vector(dimension: u32, required: bool) -> FieldSpec {
        Self::vector_field(FieldType::f16_vector(dimension), required)
    }

    pub fn f8_vector(dimension: u32, required: bool) -> FieldSpec {
        Self::vector_field(FieldType::f8_vector(dimension), required)
    }

    pub fn u8_vector(dimension: u32, required: bool) -> FieldSpec {
        Self::vector_field(FieldType::u8_vector(dimension), required)
    }

    pub fn i8_vector(dimension: u32, required: bool) -> FieldSpec {
        Self::vector_field(FieldType::i8_vector(dimension), required)
    }

    pub fn binary_vector(dimension: u32, required: bool) -> FieldSpec {
        Self::vector_field(FieldType::binary_vector(dimension), required)
    }

    pub fn f32_sparse_vector(required: bool) -> FieldSpec {
        Self::vector_field(FieldType::f32_sparse_vector(), required)
    }

    pub fn f16_sparse_vector(required: bool) -> FieldSpec {
        Self::vector_field(FieldType::f16_sparse_vector(), required)
    }

    pub fn f8_sparse_vector(required: bool) -> FieldSpec {
        Self::vector_field(FieldType::f8_sparse_vector(), required)
    }

    pub fn u8_sparse_vector(required: bool) -> FieldSpec {
        Self::vector_field(FieldType::u8_sparse_vector(), required)
    }

    pub fn i8_sparse_vector(required: bool) -> FieldSpec {
        Self::vector_field(FieldType::i8_sparse_vector(), required)
    }

    fn vector_field(data_type: FieldType, required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(data_type),
            required,
            index: None,
        }
    }

    pub fn matrix(required: bool, num_cols: u32, value_type: MatrixValueType) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::matrix(num_cols, value_type)),
            required,
            index: None,
        }
    }

    pub fn r#struct(
        required: bool,
        fields: impl IntoIterator<Item = (impl Into<String>, FieldSpec)>,
    ) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::r#struct(fields)),
            required,
            index: None,
        }
    }
}
