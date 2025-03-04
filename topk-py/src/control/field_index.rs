use pyo3::prelude::*;

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq)]
pub enum FieldIndex {
    KeywordIndex {
        index_type: KeywordIndexType,
    },
    VectorIndex {
        metric: VectorDistanceMetric,
    },
    SemanticIndex {
        model: Option<String>,
        embedding_type: Option<EmbeddingDataType>,
    },
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddingDataType {
    Float32,
    UInt8,
    Binary,
}

impl From<EmbeddingDataType> for topk_protos::v1::control::EmbeddingDataType {
    fn from(dt: EmbeddingDataType) -> Self {
        match dt {
            EmbeddingDataType::Float32 => topk_protos::v1::control::EmbeddingDataType::F32,
            EmbeddingDataType::UInt8 => topk_protos::v1::control::EmbeddingDataType::U8,
            EmbeddingDataType::Binary => topk_protos::v1::control::EmbeddingDataType::Binary,
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum VectorDistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
    Hamming,
}

impl From<VectorDistanceMetric> for topk_protos::v1::control::VectorDistanceMetric {
    fn from(metric: VectorDistanceMetric) -> Self {
        match metric {
            VectorDistanceMetric::Cosine => topk_protos::v1::control::VectorDistanceMetric::Cosine,
            VectorDistanceMetric::Euclidean => {
                topk_protos::v1::control::VectorDistanceMetric::Euclidean
            }
            VectorDistanceMetric::DotProduct => {
                topk_protos::v1::control::VectorDistanceMetric::DotProduct
            }
            VectorDistanceMetric::Hamming => {
                topk_protos::v1::control::VectorDistanceMetric::Hamming
            }
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum KeywordIndexType {
    Text,
}

impl From<KeywordIndexType> for topk_protos::v1::control::KeywordIndexType {
    fn from(index_type: KeywordIndexType) -> Self {
        match index_type {
            KeywordIndexType::Text => topk_protos::v1::control::KeywordIndexType::Text,
        }
    }
}

impl Into<topk_protos::v1::control::FieldIndex> for FieldIndex {
    fn into(self) -> topk_protos::v1::control::FieldIndex {
        match self {
            FieldIndex::KeywordIndex { index_type } => {
                topk_protos::v1::control::FieldIndex::keyword(index_type.into())
            }
            FieldIndex::VectorIndex { metric } => {
                topk_protos::v1::control::FieldIndex::vector(metric.into())
            }
            FieldIndex::SemanticIndex {
                model,
                embedding_type,
            } => topk_protos::v1::control::FieldIndex::semantic(
                model,
                embedding_type.map(|dt| dt.into()),
            ),
        }
    }
}

impl From<topk_protos::v1::control::FieldIndex> for FieldIndex {
    fn from(proto: topk_protos::v1::control::FieldIndex) -> Self {
        match proto.index.expect("index is required") {
            topk_protos::v1::control::field_index::Index::KeywordIndex(keyword_index) => {
                FieldIndex::KeywordIndex {
                    index_type: match keyword_index.index_type() {
                        topk_protos::v1::control::KeywordIndexType::Text => KeywordIndexType::Text,
                        t => panic!("unsupported keyword index: {:?}", t),
                    },
                }
            }
            topk_protos::v1::control::field_index::Index::VectorIndex(vector_index) => {
                FieldIndex::VectorIndex {
                    metric: match vector_index.metric() {
                        topk_protos::v1::control::VectorDistanceMetric::Cosine => {
                            VectorDistanceMetric::Cosine
                        }
                        topk_protos::v1::control::VectorDistanceMetric::Euclidean => {
                            VectorDistanceMetric::Euclidean
                        }
                        topk_protos::v1::control::VectorDistanceMetric::DotProduct => {
                            VectorDistanceMetric::DotProduct
                        }
                        topk_protos::v1::control::VectorDistanceMetric::Hamming => {
                            VectorDistanceMetric::Hamming
                        }
                        m => panic!("unsupported vector metric {:?}", m),
                    },
                }
            }
            topk_protos::v1::control::field_index::Index::SemanticIndex(semantic_index) => {
                let embedding_type = match semantic_index.embedding_type() {
                    topk_protos::v1::control::EmbeddingDataType::Unspecified => None,
                    topk_protos::v1::control::EmbeddingDataType::Binary => {
                        Some(EmbeddingDataType::Binary)
                    }
                    topk_protos::v1::control::EmbeddingDataType::F32 => {
                        Some(EmbeddingDataType::Float32)
                    }
                    topk_protos::v1::control::EmbeddingDataType::U8 => {
                        Some(EmbeddingDataType::UInt8)
                    }
                };
                FieldIndex::SemanticIndex {
                    model: semantic_index.model,
                    embedding_type,
                }
            }
        }
    }
}
