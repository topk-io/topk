// use crate::control::{self, field_index::EmbeddingDataType};
// use napi::bindgen_prelude::*;
// use napi_derive::napi;

// #[napi]
// pub fn text() -> control::field_spec::FieldSpec {
//     control::field_spec::FieldSpec::new(control::data_type::DataType::Text {})
// }

// #[napi]
// pub fn int() -> control::field_spec::FieldSpec {
//     control::field_spec::FieldSpec::new(control::data_type::DataType::Integer {})
// }

// #[napi]
// pub fn float() -> control::field_spec::FieldSpec {
//     control::field_spec::FieldSpec::new(control::data_type::DataType::Float {})
// }

// #[napi]
// pub fn bool() -> control::field_spec::FieldSpec {
//     control::field_spec::FieldSpec::new(control::data_type::DataType::Boolean {})
// }

// #[napi]
// pub fn f32_vector(dimension: u32) -> control::field_spec::FieldSpec {
//     control::field_spec::FieldSpec::new(control::data_type::DataType::F32Vector { dimension })
// }

// #[napi]
// pub fn u8_vector(dimension: u32) -> control::field_spec::FieldSpec {
//     control::field_spec::FieldSpec::new(control::data_type::DataType::U8Vector { dimension })
// }

// #[napi]
// pub fn binary_vector(dimension: u32) -> control::field_spec::FieldSpec {
//     control::field_spec::FieldSpec::new(control::data_type::DataType::BinaryVector { dimension })
// }

// #[napi]
// pub fn bytes() -> control::field_spec::FieldSpec {
//     control::field_spec::FieldSpec::new(control::data_type::DataType::Bytes {})
// }

// #[napi]
// pub fn vector_index(metric: String) -> Result<control::field_index::FieldIndex> {
//     let metric = match metric.to_lowercase().as_str() {
//         "cosine" => control::field_index::VectorDistanceMetric::Cosine,
//         "euclidean" => control::field_index::VectorDistanceMetric::Euclidean,
//         "dot_product" => control::field_index::VectorDistanceMetric::DotProduct,
//         "hamming" => control::field_index::VectorDistanceMetric::Hamming,
//         _ => {
//             return Err(napi::Error::new(
//                 napi::Status::GenericFailure,
//                 format!("Invalid vector distance metric: {}. Supported metrics are: cosine, euclidean, dot_product, hamming.", metric),
//             ))
//         }
//     };

//     Ok(control::field_index::FieldIndex::VectorIndex { metric })
// }

// #[napi]
// pub fn keyword_index(typ: String) -> Result<control::field_index::FieldIndex> {
//     let index_type = match typ.to_lowercase().as_str() {
//         "text" => control::field_index::KeywordIndexType::Text,
//         _ => {
//             return Err(napi::Error::new(
//                 napi::Status::GenericFailure,
//                 format!(
//                     "Invalid keyword index type: {}. Supported index types are: text.",
//                     typ
//                 ),
//             ))
//         }
//     };

//     Ok(control::field_index::FieldIndex::KeywordIndex { index_type })
// }

// #[napi]
// pub fn semantic_index(model: Option<String>) -> Result<control::field_index::FieldIndex> {
//     Ok(control::field_index::FieldIndex::SemanticIndex {
//         model,
//         embedding_type: None,
//     })
// }
