use crate::error::RustError;
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
    MultiVectorIndex {
        metric: MultiVectorDistanceMetric,
        sketch_bits: Option<u32>,
        quantization: Option<MultiVectorQuantization>,
    },
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddingDataType {
    Float32,
    UInt8,
    Binary,
}

impl From<EmbeddingDataType> for topk_rs::proto::v1::control::EmbeddingDataType {
    fn from(dt: EmbeddingDataType) -> Self {
        match dt {
            EmbeddingDataType::Float32 => topk_rs::proto::v1::control::EmbeddingDataType::F32,
            EmbeddingDataType::UInt8 => topk_rs::proto::v1::control::EmbeddingDataType::U8,
            EmbeddingDataType::Binary => topk_rs::proto::v1::control::EmbeddingDataType::Binary,
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

impl TryFrom<topk_rs::proto::v1::control::VectorDistanceMetric> for VectorDistanceMetric {
    type Error = RustError;

    fn try_from(
        metric: topk_rs::proto::v1::control::VectorDistanceMetric,
    ) -> Result<Self, Self::Error> {
        match metric {
            topk_rs::proto::v1::control::VectorDistanceMetric::Cosine => {
                Ok(VectorDistanceMetric::Cosine)
            }
            topk_rs::proto::v1::control::VectorDistanceMetric::Euclidean => {
                Ok(VectorDistanceMetric::Euclidean)
            }
            topk_rs::proto::v1::control::VectorDistanceMetric::DotProduct => {
                Ok(VectorDistanceMetric::DotProduct)
            }
            topk_rs::proto::v1::control::VectorDistanceMetric::Hamming => {
                Ok(VectorDistanceMetric::Hamming)
            }
            topk_rs::proto::v1::control::VectorDistanceMetric::Unspecified => {
                Err(topk_rs::Error::InvalidProto.into())
            }
        }
    }
}

impl From<VectorDistanceMetric> for topk_rs::proto::v1::control::VectorDistanceMetric {
    fn from(metric: VectorDistanceMetric) -> Self {
        match metric {
            VectorDistanceMetric::Cosine => {
                topk_rs::proto::v1::control::VectorDistanceMetric::Cosine
            }
            VectorDistanceMetric::Euclidean => {
                topk_rs::proto::v1::control::VectorDistanceMetric::Euclidean
            }
            VectorDistanceMetric::DotProduct => {
                topk_rs::proto::v1::control::VectorDistanceMetric::DotProduct
            }
            VectorDistanceMetric::Hamming => {
                topk_rs::proto::v1::control::VectorDistanceMetric::Hamming
            }
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum KeywordIndexType {
    Text,
}

impl From<KeywordIndexType> for topk_rs::proto::v1::control::KeywordIndexType {
    fn from(index_type: KeywordIndexType) -> Self {
        match index_type {
            KeywordIndexType::Text => topk_rs::proto::v1::control::KeywordIndexType::Text,
        }
    }
}

impl TryFrom<topk_rs::proto::v1::control::KeywordIndexType> for KeywordIndexType {
    type Error = RustError;

    fn try_from(
        index_type: topk_rs::proto::v1::control::KeywordIndexType,
    ) -> Result<Self, Self::Error> {
        match index_type {
            topk_rs::proto::v1::control::KeywordIndexType::Text => Ok(KeywordIndexType::Text),
            topk_rs::proto::v1::control::KeywordIndexType::Unspecified => {
                Err(topk_rs::Error::InvalidProto.into())
            }
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum MultiVectorDistanceMetric {
    Maxsim,
}

impl From<MultiVectorDistanceMetric> for topk_rs::proto::v1::control::MultiVectorDistanceMetric {
    fn from(metric: MultiVectorDistanceMetric) -> Self {
        match metric {
            MultiVectorDistanceMetric::Maxsim => {
                topk_rs::proto::v1::control::MultiVectorDistanceMetric::Maxsim
            }
        }
    }
}

impl TryFrom<topk_rs::proto::v1::control::MultiVectorDistanceMetric> for MultiVectorDistanceMetric {
    type Error = RustError;

    fn try_from(
        metric: topk_rs::proto::v1::control::MultiVectorDistanceMetric,
    ) -> Result<Self, Self::Error> {
        match metric {
            topk_rs::proto::v1::control::MultiVectorDistanceMetric::Maxsim => {
                Ok(MultiVectorDistanceMetric::Maxsim)
            }
            topk_rs::proto::v1::control::MultiVectorDistanceMetric::Unspecified => {
                Err(topk_rs::Error::InvalidProto.into())
            }
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum MultiVectorQuantization {
    Binary1bit,
    Binary2bit,
    Scalar,
}

impl From<MultiVectorQuantization> for topk_rs::proto::v1::control::MultiVectorQuantization {
    fn from(metric: MultiVectorQuantization) -> Self {
        match metric {
            MultiVectorQuantization::Binary1bit => {
                topk_rs::proto::v1::control::MultiVectorQuantization::Binary1bit
            }
            MultiVectorQuantization::Binary2bit => {
                topk_rs::proto::v1::control::MultiVectorQuantization::Binary2bit
            }
            MultiVectorQuantization::Scalar => {
                topk_rs::proto::v1::control::MultiVectorQuantization::Scalar
            }
        }
    }
}

impl Into<topk_rs::proto::v1::control::FieldIndex> for FieldIndex {
    fn into(self) -> topk_rs::proto::v1::control::FieldIndex {
        match self {
            FieldIndex::KeywordIndex { index_type } => {
                topk_rs::proto::v1::control::FieldIndex::keyword(index_type.into())
            }
            FieldIndex::VectorIndex { metric } => {
                topk_rs::proto::v1::control::FieldIndex::vector(metric.into())
            }
            FieldIndex::SemanticIndex {
                model,
                embedding_type,
            } => topk_rs::proto::v1::control::FieldIndex::semantic(
                model,
                embedding_type.map(|dt| dt.into()),
            ),
            FieldIndex::MultiVectorIndex {
                metric,
                sketch_bits,
                quantization,
            } => topk_rs::proto::v1::control::FieldIndex::multi_vector(
                metric.into(),
                sketch_bits,
                quantization.map(|q| q.into()),
            ),
        }
    }
}

impl TryFrom<topk_rs::proto::v1::control::FieldIndex> for FieldIndex {
    type Error = RustError;

    fn try_from(proto: topk_rs::proto::v1::control::FieldIndex) -> Result<Self, Self::Error> {
        let index = proto.index.ok_or(topk_rs::Error::InvalidProto)?;
        Ok(match index {
            topk_rs::proto::v1::control::field_index::Index::KeywordIndex(keyword_index) => {
                FieldIndex::KeywordIndex {
                    index_type: keyword_index.index_type().try_into()?,
                }
            }
            topk_rs::proto::v1::control::field_index::Index::VectorIndex(vector_index) => {
                FieldIndex::VectorIndex {
                    metric: vector_index.metric().try_into()?,
                }
            }
            topk_rs::proto::v1::control::field_index::Index::SemanticIndex(semantic_index) => {
                let embedding_type = match semantic_index.embedding_type() {
                    topk_rs::proto::v1::control::EmbeddingDataType::Unspecified => None,
                    topk_rs::proto::v1::control::EmbeddingDataType::Binary => {
                        Some(EmbeddingDataType::Binary)
                    }
                    topk_rs::proto::v1::control::EmbeddingDataType::F32 => {
                        Some(EmbeddingDataType::Float32)
                    }
                    topk_rs::proto::v1::control::EmbeddingDataType::U8 => {
                        Some(EmbeddingDataType::UInt8)
                    }
                };
                FieldIndex::SemanticIndex {
                    model: semantic_index.model,
                    embedding_type,
                }
            }
            topk_rs::proto::v1::control::field_index::Index::MultiVectorIndex(mvi) => {
                FieldIndex::MultiVectorIndex {
                    metric: mvi.metric().try_into()?,
                    sketch_bits: mvi.sketch_bits,
                    quantization: match mvi.quantization() {
                        topk_rs::proto::v1::control::MultiVectorQuantization::Binary1bit => {
                            Some(MultiVectorQuantization::Binary1bit)
                        }
                        topk_rs::proto::v1::control::MultiVectorQuantization::Binary2bit => {
                            Some(MultiVectorQuantization::Binary2bit)
                        }
                        topk_rs::proto::v1::control::MultiVectorQuantization::Scalar => {
                            Some(MultiVectorQuantization::Scalar)
                        }
                        _ => None,
                    },
                }
            }
        })
    }
}
