use std::collections::HashMap;

use crate::proto::control::v1::{
    field_type_list::ListValueType, field_type_matrix::MatrixValueType,
};

use super::*;

impl FieldSpec {
    pub fn index(&self) -> Option<&field_index::Index> {
        self.index.as_ref()?.index.as_ref()
    }

    pub fn data_type(&self) -> Option<&field_type::DataType> {
        self.data_type.as_ref()?.data_type.as_ref()
    }

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

    pub fn timestamp(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::timestamp()),
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

    pub fn image(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType::image()),
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

    pub fn as_struct(&self) -> Option<&HashMap<String, FieldSpec>> {
        match self.data_type()? {
            field_type::DataType::Struct(s) => Some(&s.fields),
            _ => None,
        }
    }

    pub fn as_image(&self) -> Option<&FieldTypeImage> {
        match self.data_type.as_ref()?.data_type.as_ref()? {
            field_type::DataType::Image(i) => Some(i),
            _ => None,
        }
    }

    pub fn as_matrix(&self) -> Option<&FieldTypeMatrix> {
        match self.data_type.as_ref()?.data_type.as_ref()? {
            field_type::DataType::Matrix(m) => Some(m),
            _ => None,
        }
    }
}
