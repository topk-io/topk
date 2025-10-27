pub mod data_type;
pub mod field_index;
pub mod field_spec;

use data_type::DataType;
use field_index::{EmbeddingDataType, FieldIndex, KeywordIndexType, VectorDistanceMetric};
use field_spec::FieldSpec;
use napi_derive::napi;

use crate::schema::data_type::ListValueType;

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `text` values.
///
/// Example:
///
/// ```javascript
/// import { text } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   title: text()
/// });
/// ```
#[napi(namespace = "schema")]
pub fn text() -> FieldSpec {
    FieldSpec::create(DataType::Text)
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `int` values.
///
/// Example:
///
/// ```javascript
/// import { int } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   published_year: int()
/// });
/// ```
#[napi(namespace = "schema")]
pub fn int() -> FieldSpec {
    FieldSpec::create(DataType::Integer {})
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `float` values.
///
/// Example:
///
/// ```javascript
/// import { float } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   price: float()
/// });
/// ```
#[napi(namespace = "schema")]
pub fn float() -> FieldSpec {
    FieldSpec::create(DataType::Float {})
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `bool` values.
///
/// Example:
///
/// ```javascript
/// import { bool } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   is_published: bool()
/// });
/// ```
#[napi(namespace = "schema")]
pub fn bool() -> FieldSpec {
    FieldSpec::create(DataType::Boolean {})
}

/// Options for vector field specifications.
///
/// This struct contains configuration options for vector fields,
/// including the required dimension parameter.
#[napi(object, namespace = "schema")]
pub struct VectorOptions {
    /// The dimension of the vector
    pub dimension: u32,
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `f32_vector` values.
///
/// Example:
///
/// ```javascript
/// import { f32Vector } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   title_embedding: f32Vector({ dimension: 1536 })
/// });
/// ```
#[napi(namespace = "schema")]
pub fn f32_vector(options: VectorOptions) -> FieldSpec {
    FieldSpec::create(DataType::F32Vector {
        dimension: options.dimension,
    })
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `u8_vector` values.
///
/// Example:
///
/// ```javascript
/// import { u8Vector } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   title_embedding: u8Vector({ dimension: 1536 })
/// });
/// ```
#[napi(namespace = "schema")]
pub fn u8_vector(options: VectorOptions) -> FieldSpec {
    FieldSpec::create(DataType::U8Vector {
        dimension: options.dimension,
    })
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `i8_vector` values.
///
/// Example:
///
/// ```javascript
/// import { i8Vector } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   title_embedding: i8Vector({ dimension: 1536 })
/// });
/// ```
#[napi(namespace = "schema")]
pub fn i8_vector(options: VectorOptions) -> FieldSpec {
    FieldSpec::create(DataType::I8Vector {
        dimension: options.dimension,
    })
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `binary_vector` values.
///
/// Example:
///
/// ```javascript
/// import { binaryVector } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   title_embedding: binaryVector({ dimension: 128 })
/// });
/// ```
#[napi(namespace = "schema")]
pub fn binary_vector(options: VectorOptions) -> FieldSpec {
    FieldSpec::create(DataType::BinaryVector {
        dimension: options.dimension,
    })
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `bytes` values.
///
/// Example:
///
/// ```javascript
/// import { bytes } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   image: bytes()
/// });
/// ```
#[napi(namespace = "schema")]
pub fn bytes() -> FieldSpec {
    FieldSpec::create(DataType::Bytes {})
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `f32_sparse_vector` values.
///
/// Note: Sparse vectors use u32 dimension indices to support dictionaries of up to 2^32 - 1 terms.
///
/// Example:
///
/// ```javascript
/// import { f32SparseVector } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   sparse_field: f32SparseVector()
/// });
/// ```
#[napi(namespace = "schema")]
pub fn f32_sparse_vector() -> FieldSpec {
    FieldSpec::create(DataType::F32SparseVector {})
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `u8_sparse_vector` values.
///
/// Note: Sparse vectors use u32 dimension indices to support dictionaries of up to 2^32 - 1 terms.
///
/// Example:
///
/// ```javascript
/// import { u8SparseVector } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   sparse_field: u8SparseVector()
/// });
/// ```
#[napi(namespace = "schema")]
pub fn u8_sparse_vector() -> FieldSpec {
    FieldSpec::create(DataType::U8SparseVector {})
}

/// Options for vector index specifications.
///
/// This struct contains configuration options for vector indexes,
/// including the distance metric to use.
#[napi(object, namespace = "schema")]
pub struct VectorIndexOptions {
    /// The distance metric to use for vector similarity
    pub metric: VectorDistanceMetric,
}

/// Creates a [FieldIndex](https://docs.topk.io/sdk/topk-js/schema#FieldIndex) type for `vector_index` values.
///
/// Supported `metric`s:
/// - `euclidean` (not supported for sparse vectors)
/// - `cosine` (not supported for sparse vectors)
/// - `dot_product` (supported for dense and sparse vectors)
/// - `hamming` (only supported for binary_vector type)
///
/// Example:
///
/// ```javascript
/// import { f32Vector, vectorIndex } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   title_embedding: f32Vector({ dimension: 1536 }).index(
///     vectorIndex({ metric: "cosine" })
///   )
/// });
/// ```
#[napi(namespace = "schema")]
pub fn vector_index(options: VectorIndexOptions) -> FieldIndex {
    FieldIndex::vector_index(options.metric)
}

/// Creates a [FieldIndex](https://docs.topk.io/sdk/topk-js/schema#FieldIndex) type for `keyword_index` values.
///
/// Example:
///
/// ```javascript
/// import { text, keywordIndex } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   title: text().index(keywordIndex())
/// });
/// ```
///
/// Adding a keyword index allows you to perform keyword search on this field.
#[napi(namespace = "schema")]
pub fn keyword_index() -> FieldIndex {
    FieldIndex::keyword_index(KeywordIndexType::Text)
}

/// Options for semantic index specifications.
///
/// This struct contains configuration options for semantic indexes,
/// including the model and embedding type to use.
#[napi(object, namespace = "schema")]
#[derive(Default)]
pub struct SemanticIndexOptions {
    /// The embedding model to use
    pub model: Option<String>,
    /// The type of embedding data
    pub embedding_type: Option<EmbeddingDataType>,
}

/// Creates a [FieldIndex](https://docs.topk.io/sdk/topk-js/schema#FieldIndex) type for `semantic_index` values.
///
/// Example:
///
/// ```javascript
/// import { text, semanticIndex } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   title: text().index(semanticIndex({ model: "cohere/embed-v4" }))
/// });
/// ```
///
/// Parameters:
/// - model: Embedding model to use for semantic search. Currently supported:
///   - `cohere/embed-english-v3`
///   - `cohere/embed-multilingual-v3`
///   - `cohere/embed-v4` (default)
/// - embeddingType: TopK supports the following embedding types for Cohere models:
///   - `float32`
///   - `uint8`
///   - `binary`
#[napi(namespace = "schema")]
pub fn semantic_index(options: Option<SemanticIndexOptions>) -> FieldIndex {
    let options = options.unwrap_or_default();

    FieldIndex::semantic_index(options.model, options.embedding_type)
}

/// Options for list field specifications.
///
/// This struct contains configuration options for list fields,
/// including the type of values the list can contain.
#[napi(object, namespace = "schema")]
pub struct ListOptions {
    /// The type of values the list can contain
    pub value_type: ListValueType,
}

/// Creates a [FieldSpec](https://docs.topk.io/sdk/topk-js/schema#FieldSpec) type for `list` values.
///
/// Example:
///
/// ```javascript
/// import { list } from "topk-js/schema";
///
/// await client.collections().create("books", {
///   tags: list({ valueType: "text" })
/// });
/// ```
#[napi(namespace = "schema")]
pub fn list(options: ListOptions) -> FieldSpec {
    FieldSpec::create(DataType::List {
        value_type: options.value_type,
    })
}
