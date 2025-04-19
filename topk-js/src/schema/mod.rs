pub mod data_type;
pub mod field_index;
pub mod field_spec;

use data_type::DataType;
use field_index::{
    EmbeddingDataType, FieldIndex, FieldIndexUnion, KeywordIndexType, VectorDistanceMetric,
};
use field_spec::FieldSpec;
use napi_derive::napi;

#[napi(namespace = "schema")]
pub fn text() -> FieldSpec {
    FieldSpec::create(DataType::Text)
}

#[napi(namespace = "schema")]
pub fn int() -> FieldSpec {
    FieldSpec::create(DataType::Integer {})
}

#[napi(namespace = "schema")]
pub fn float() -> FieldSpec {
    FieldSpec::create(DataType::Float {})
}

#[napi(namespace = "schema")]
pub fn bool() -> FieldSpec {
    FieldSpec::create(DataType::Boolean {})
}

#[napi(namespace = "schema")]
pub fn f32_vector(dimension: u32) -> FieldSpec {
    FieldSpec::create(DataType::F32Vector { dimension })
}

#[napi(namespace = "schema")]
pub fn u8_vector(dimension: u32) -> FieldSpec {
    FieldSpec::create(DataType::U8Vector { dimension })
}

#[napi(namespace = "schema")]
pub fn binary_vector(dimension: u32) -> FieldSpec {
    FieldSpec::create(DataType::BinaryVector { dimension })
}

#[napi(namespace = "schema")]
pub fn bytes() -> FieldSpec {
    FieldSpec::create(DataType::Bytes {})
}

#[napi(object, namespace = "schema")]
pub struct VectorIndexOptions {
    pub metric: VectorDistanceMetric,
}

#[napi(namespace = "schema")]
pub fn vector_index(options: VectorIndexOptions) -> FieldIndex {
    FieldIndex {
        index: Some(FieldIndexUnion::VectorIndex {
            metric: options.metric,
        }),
    }
}

#[napi(namespace = "schema")]
pub fn keyword_index() -> FieldIndex {
    FieldIndex {
        index: Some(FieldIndexUnion::KeywordIndex {
            index_type: KeywordIndexType::Text,
        }),
    }
}

#[napi(object, namespace = "schema")]
pub struct SemanticIndexOptions {
    pub model: Option<String>,
    pub embedding_type: Option<EmbeddingDataType>,
}

#[napi(namespace = "schema")]
pub fn semantic_index(options: SemanticIndexOptions) -> FieldIndex {
    FieldIndex {
        index: Some(FieldIndexUnion::SemanticIndex {
            model: options.model,
            embedding_type: options.embedding_type,
        }),
    }
}
