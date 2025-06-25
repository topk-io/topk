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
pub fn semantic_index(options: Option<SemanticIndexOptions>) -> FieldIndex {
    let (model, embedding_type) = match options {
        Some(options) => (options.model, options.embedding_type),
        None => (None, None),
    };

    FieldIndex {
        index: Some(FieldIndexUnion::SemanticIndex {
            model,
            embedding_type,
        }),
    }
}
