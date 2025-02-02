include!(concat!(env!("OUT_DIR"), "/topk.control.v1.rs"));

pub mod collection_ext;
pub mod data_type_ext;
pub mod field_index_ext;
pub mod field_spec_ext;

impl field_type::DataType {
    pub fn to_user_friendly_type_name(&self) -> String {
        match self {
            field_type::DataType::Text(..) => "text".to_string(),
            field_type::DataType::Integer(..) => "integer".to_string(),
            field_type::DataType::Float(..) => "float".to_string(),
            field_type::DataType::Boolean(..) => "boolean".to_string(),
            field_type::DataType::F32Vector(..) => "f32_vector".to_string(),
            field_type::DataType::U8Vector(..) => "u8_vector".to_string(),
            field_type::DataType::BinaryVector(..) => "binary_vector".to_string(),
            field_type::DataType::Bytes(..) => "bytes".to_string(),
        }
    }
}
