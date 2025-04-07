use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub enum FieldIndex {
    Keyword,
    Vector {
        metric: VectorFieldIndexMetric,
    },
    Semantic {
        model: Option<String>,
        embedding_type: EmbeddingDataType,
    },
}

#[napi(string_enum)]
pub enum VectorFieldIndexMetric {
    Cosine,
    Euclidean,
    DotProduct,
    Hamming,
}

impl From<i32> for VectorFieldIndexMetric {
    fn from(metric: i32) -> Self {
        match metric {
            1 => VectorFieldIndexMetric::Cosine,
            2 => VectorFieldIndexMetric::Euclidean,
            3 => VectorFieldIndexMetric::DotProduct,
            4 => VectorFieldIndexMetric::Hamming,
            _ => unreachable!("Unsupported vector field index metric"),
        }
    }
}

impl From<VectorFieldIndexMetric> for i32 {
    fn from(metric: VectorFieldIndexMetric) -> Self {
        match metric {
            VectorFieldIndexMetric::Cosine => 1,
            VectorFieldIndexMetric::Euclidean => 2,
            VectorFieldIndexMetric::DotProduct => 3,
            VectorFieldIndexMetric::Hamming => 4,
        }
    }
}

#[napi(string_enum)]
pub enum EmbeddingDataType {
    F32,
    U8,
    /// Binary quantized uint8
    Binary,
}

impl From<Option<i32>> for EmbeddingDataType {
    fn from(embedding_type: Option<i32>) -> Self {
        match embedding_type {
            Some(0) => EmbeddingDataType::F32,
            Some(1) => EmbeddingDataType::U8,
            Some(2) => EmbeddingDataType::Binary,
            _ => unreachable!("Unsupported embedding data type"),
        }
    }
}

impl From<EmbeddingDataType> for Option<i32> {
    fn from(embedding_type: EmbeddingDataType) -> Self {
        match embedding_type {
            EmbeddingDataType::F32 => Some(0),
            EmbeddingDataType::U8 => Some(1),
            EmbeddingDataType::Binary => Some(2),
        }
    }
}

impl From<FieldIndex> for topk_protos::v1::control::FieldIndex {
    fn from(field_index: FieldIndex) -> Self {
        Self {
            index: Some(match field_index {
                FieldIndex::Keyword => topk_protos::v1::control::field_index::Index::KeywordIndex(
                    topk_protos::v1::control::KeywordIndex {
                        index_type: topk_protos::v1::control::KeywordIndexType::Text.into(),
                    },
                ),
                FieldIndex::Vector { metric } => {
                    topk_protos::v1::control::field_index::Index::VectorIndex(
                        topk_protos::v1::control::VectorIndex {
                            metric: metric.into(),
                        },
                    )
                }
                FieldIndex::Semantic {
                    model,
                    embedding_type,
                } => topk_protos::v1::control::field_index::Index::SemanticIndex(
                    topk_protos::v1::control::SemanticIndex {
                        model,
                        embedding_type: embedding_type.into(),
                    },
                ),
            }),
        }
    }
}

impl From<topk_protos::v1::control::FieldIndex> for FieldIndex {
    fn from(field_index: topk_protos::v1::control::FieldIndex) -> Self {
        match field_index.index.unwrap_or_else(|| {
            topk_protos::v1::control::field_index::Index::KeywordIndex(
                topk_protos::v1::control::KeywordIndex {
                    index_type: topk_protos::v1::control::KeywordIndexType::Text.into(),
                },
            )
        }) {
            topk_protos::v1::control::field_index::Index::KeywordIndex(_k) => {
                FieldIndex::Keyword {}
            }
            topk_protos::v1::control::field_index::Index::VectorIndex(v) => FieldIndex::Vector {
                metric: v.metric.into(),
            },
            topk_protos::v1::control::field_index::Index::SemanticIndex(s) => {
                FieldIndex::Semantic {
                    model: s.model,
                    embedding_type: s.embedding_type.into(),
                }
            }
        }
    }
}
