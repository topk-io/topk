use crate::control::{
    self,
    field_index::{EmbeddingDataType, VectorDistanceMetric},
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(namespace = "schema")]
pub fn text() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Text {})
}

#[napi(namespace = "schema")]
pub fn int() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Integer {})
}

#[napi(namespace = "schema")]
pub fn float() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Float {})
}

#[napi(namespace = "schema")]
pub fn bool() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Boolean {})
}

#[napi(namespace = "schema")]
pub fn f32_vector(dimension: u32) -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::F32Vector { dimension })
}

#[napi(namespace = "schema")]
pub fn u8_vector(dimension: u32) -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::U8Vector { dimension })
}

#[napi(namespace = "schema")]
pub fn binary_vector(dimension: u32) -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::BinaryVector { dimension })
}

#[napi(namespace = "schema")]
pub fn bytes() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Bytes {})
}

#[napi(object, namespace = "schema")]
pub struct VectorIndexOptions {
    pub metric: VectorDistanceMetric,
}

#[napi(namespace = "schema")]
pub fn vector_index(options: VectorIndexOptions) -> Result<control::field_index::FieldIndex> {
    Ok(control::field_index::FieldIndex::VectorIndex {
        metric: options.metric,
    })
}

#[napi(namespace = "schema")]
pub fn keyword_index() -> Result<control::field_index::FieldIndex> {
    Ok(control::field_index::FieldIndex::KeywordIndex)
}

#[napi(object, namespace = "schema")]
pub struct SemanticIndexOptions {
    pub model: Option<String>,
    pub embedding_type: Option<EmbeddingDataType>,
}

#[napi(namespace = "schema")]
pub fn semantic_index(options: SemanticIndexOptions) -> Result<control::field_index::FieldIndex> {
    Ok(control::field_index::FieldIndex::SemanticIndex {
        model: options.model,
        embedding_type: options.embedding_type,
    })
}
