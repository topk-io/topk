use std::collections::HashMap;

use crate::control::{
    self,
    field_index::{EmbeddingDataType, VectorDistanceMetric},
    field_spec::FieldSpec,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

pub struct Schema(HashMap<String, FieldSpec>);

impl FromNapiValue for Schema {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self> {
        let map = HashMap::from_napi_value(env, value)?;
        Ok(Schema(map))
    }
}

#[napi]
pub fn text() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Text {})
}

#[napi]
pub fn int() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Integer {})
}

#[napi]
pub fn float() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Float {})
}

#[napi]
pub fn bool() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Boolean {})
}

#[napi]
pub fn f32_vector(dimension: u32) -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::F32Vector { dimension })
}

#[napi]
pub fn u8_vector(dimension: u32) -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::U8Vector { dimension })
}

#[napi]
pub fn binary_vector(dimension: u32) -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::BinaryVector { dimension })
}

#[napi]
pub fn bytes() -> control::field_spec::FieldSpec {
    control::field_spec::FieldSpec::create(control::data_type::DataType::Bytes {})
}

#[napi(object)]
pub struct VectorIndexOptions {
    pub metric: VectorDistanceMetric,
}

#[napi]
pub fn vector_index(options: VectorIndexOptions) -> Result<control::field_index::FieldIndex> {
    Ok(control::field_index::FieldIndex::VectorIndex {
        metric: options.metric,
    })
}

#[napi]
pub fn keyword_index() -> Result<control::field_index::FieldIndex> {
    Ok(control::field_index::FieldIndex::KeywordIndex)
}

#[napi(object)]
pub struct SemanticIndexOptions {
    pub model: Option<String>,
    pub embedding_type: Option<EmbeddingDataType>,
}

#[napi]
pub fn semantic_index(options: SemanticIndexOptions) -> Result<control::field_index::FieldIndex> {
    Ok(control::field_index::FieldIndex::SemanticIndex {
        model: options.model,
        embedding_type: options.embedding_type,
    })
}
