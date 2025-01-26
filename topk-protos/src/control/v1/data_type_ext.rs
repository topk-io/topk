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

    pub fn float_vector(dimension: u32) -> Self {
        field_type::DataType::FloatVector(FieldTypeFloatVector { dimension })
    }

    pub fn byte_vector(dimension: u32) -> Self {
        field_type::DataType::ByteVector(FieldTypeByteVector { dimension })
    }

    pub fn bytes() -> Self {
        field_type::DataType::Bytes(FieldTypeBytes {})
    }
}
