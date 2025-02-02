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

    pub fn u8_vector(dimension: u32) -> Self {
        field_type::DataType::U8Vector(FieldTypeU8Vector { dimension })
    }

    pub fn binary_vector(dimension: u32) -> Self {
        field_type::DataType::BinaryVector(FieldTypeBinaryVector { dimension })
    }

    pub fn bytes() -> Self {
        field_type::DataType::Bytes(FieldTypeBytes {})
    }
}
