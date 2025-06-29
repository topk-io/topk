pub mod data_type;
pub mod field_index;
pub mod field_spec;

use data_type::DataType;
use field_index::{EmbeddingDataType, FieldIndex, KeywordIndexType, VectorDistanceMetric};
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

#[napi(object, namespace = "schema")]
pub struct VectorOptions {
    pub dimension: u32,
}

#[napi(namespace = "schema")]
pub fn f32_vector(options: VectorOptions) -> FieldSpec {
    FieldSpec::create(DataType::F32Vector {
        dimension: options.dimension,
    })
}

#[napi(namespace = "schema")]
pub fn u8_vector(options: VectorOptions) -> FieldSpec {
    FieldSpec::create(DataType::U8Vector {
        dimension: options.dimension,
    })
}

#[napi(namespace = "schema")]
pub fn binary_vector(options: VectorOptions) -> FieldSpec {
    FieldSpec::create(DataType::BinaryVector {
        dimension: options.dimension,
    })
}

#[napi(namespace = "schema")]
pub fn bytes() -> FieldSpec {
    FieldSpec::create(DataType::Bytes {})
}

#[napi(namespace = "schema")]
pub fn f32_sparse_vector() -> FieldSpec {
    FieldSpec::create(DataType::F32SparseVector {})
}

#[napi(namespace = "schema")]
pub fn u8_sparse_vector() -> FieldSpec {
    FieldSpec::create(DataType::U8SparseVector {})
}

#[napi(object, namespace = "schema")]
pub struct VectorIndexOptions {
    pub metric: VectorDistanceMetric,
}

#[napi(namespace = "schema")]
pub fn vector_index(options: VectorIndexOptions) -> FieldIndex {
    FieldIndex::vector_index(options.metric)
}

#[napi(namespace = "schema")]
pub fn keyword_index() -> FieldIndex {
    FieldIndex::keyword_index(KeywordIndexType::Text)
}

#[napi(object, namespace = "schema")]
#[derive(Default)]
pub struct SemanticIndexOptions {
    pub model: Option<String>,
    pub embedding_type: Option<EmbeddingDataType>,
}

#[napi(namespace = "schema")]
pub fn semantic_index(options: Option<SemanticIndexOptions>) -> FieldIndex {
    let options = options.unwrap_or_default();

    FieldIndex::semantic_index(options.model, options.embedding_type)
}
