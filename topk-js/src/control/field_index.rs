use napi_derive::napi;

#[napi]
#[derive(Clone, Debug)]
pub enum FieldIndex {
    KeywordIndex,
    VectorIndex {
        metric: VectorDistanceMetric,
    },
    SemanticIndex {
        model: Option<String>,
        embedding_type: Option<EmbeddingDataType>,
    },
}

#[napi(string_enum)]
#[derive(Clone)]
pub enum KeywordIndexType {
    Text,
}

#[napi(string_enum)]
#[derive(Clone, Debug)]
pub enum VectorDistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
    Hamming,
}

impl From<i32> for VectorDistanceMetric {
    fn from(metric: i32) -> Self {
        match metric {
            1 => VectorDistanceMetric::Cosine,
            2 => VectorDistanceMetric::Euclidean,
            3 => VectorDistanceMetric::DotProduct,
            4 => VectorDistanceMetric::Hamming,
            _ => unreachable!("Unsupported vector field index metric"),
        }
    }
}

impl From<VectorDistanceMetric> for i32 {
    fn from(metric: VectorDistanceMetric) -> Self {
        match metric {
            VectorDistanceMetric::Cosine => {
                topk_protos::v1::control::VectorDistanceMetric::Cosine.into()
            }
            VectorDistanceMetric::Euclidean => {
                topk_protos::v1::control::VectorDistanceMetric::Euclidean.into()
            }
            VectorDistanceMetric::DotProduct => {
                topk_protos::v1::control::VectorDistanceMetric::DotProduct.into()
            }
            VectorDistanceMetric::Hamming => {
                topk_protos::v1::control::VectorDistanceMetric::Hamming.into()
            }
        }
    }
}

#[napi(string_enum)]
#[derive(Clone, Debug)]
pub enum EmbeddingDataType {
    Float32,
    UInt8,
    Binary,
}

impl From<topk_protos::v1::control::EmbeddingDataType> for EmbeddingDataType {
    fn from(embedding_type: topk_protos::v1::control::EmbeddingDataType) -> Self {
        match embedding_type {
            topk_protos::v1::control::EmbeddingDataType::F32 => EmbeddingDataType::Float32,
            topk_protos::v1::control::EmbeddingDataType::U8 => EmbeddingDataType::UInt8,
            topk_protos::v1::control::EmbeddingDataType::Binary => EmbeddingDataType::Binary,
            _ => unreachable!("Unsupported embedding data type"),
        }
    }
}

impl From<EmbeddingDataType> for topk_protos::v1::control::EmbeddingDataType {
    fn from(embedding_type: EmbeddingDataType) -> Self {
        match embedding_type {
            EmbeddingDataType::Float32 => topk_protos::v1::control::EmbeddingDataType::F32.into(),
            EmbeddingDataType::UInt8 => topk_protos::v1::control::EmbeddingDataType::U8.into(),
            EmbeddingDataType::Binary => topk_protos::v1::control::EmbeddingDataType::Binary.into(),
        }
    }
}

impl From<EmbeddingDataType> for Option<i32> {
    fn from(embedding_type: EmbeddingDataType) -> Self {
        Some(match embedding_type {
            EmbeddingDataType::Float32 => topk_protos::v1::control::EmbeddingDataType::F32.into(),
            EmbeddingDataType::UInt8 => topk_protos::v1::control::EmbeddingDataType::U8.into(),
            EmbeddingDataType::Binary => topk_protos::v1::control::EmbeddingDataType::Binary.into(),
        })
    }
}

impl From<FieldIndex> for topk_protos::v1::control::FieldIndex {
    fn from(field_index: FieldIndex) -> Self {
        Self {
            index: Some(match field_index {
                FieldIndex::KeywordIndex => {
                    topk_protos::v1::control::field_index::Index::KeywordIndex(
                        topk_protos::v1::control::KeywordIndex {
                            index_type: topk_protos::v1::control::KeywordIndexType::Text.into(),
                        },
                    )
                }
                FieldIndex::VectorIndex { metric } => {
                    topk_protos::v1::control::field_index::Index::VectorIndex(
                        topk_protos::v1::control::VectorIndex {
                            metric: metric.into(),
                        },
                    )
                }
                FieldIndex::SemanticIndex {
                    model,
                    embedding_type,
                } => topk_protos::v1::control::field_index::Index::SemanticIndex(
                    topk_protos::v1::control::SemanticIndex {
                        model,
                        embedding_type: embedding_type.map(|t| t.into()).unwrap_or_default(),
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
                FieldIndex::KeywordIndex {}
            }
            topk_protos::v1::control::field_index::Index::VectorIndex(v) => {
                FieldIndex::VectorIndex {
                    metric: v.metric.into(),
                }
            }
            topk_protos::v1::control::field_index::Index::SemanticIndex(s) => {
                FieldIndex::SemanticIndex {
                    model: s.model,
                    embedding_type: None,
                }
            }
        }
    }
}
