use pyo3::prelude::*;

use crate::data::unknown::UNSUPPORTED;

use topk_rs::proto::v1::control::MultiVectorQuantization as MultiVectorQuantizationPb;

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq)]
pub enum FieldIndex {
    KeywordIndex {
        index_type: KeywordIndexType,
    },
    VectorIndex {
        metric: VectorDistanceMetric,
    },
    SemanticIndex {},
    NGramIndex {},
    MultiVectorIndex {
        metric: MultiVectorDistanceMetric,
        quantization: Option<MultiVectorQuantization>,
        width: Option<u32>,
        top_k: Option<u32>,
    },
    Unknown(),
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum VectorDistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
    Hamming,
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
    Exact,
}

impl From<KeywordIndexType> for topk_rs::proto::v1::control::KeywordIndexType {
    fn from(index_type: KeywordIndexType) -> Self {
        match index_type {
            KeywordIndexType::Text => topk_rs::proto::v1::control::KeywordIndexType::Text,
            KeywordIndexType::Exact => topk_rs::proto::v1::control::KeywordIndexType::Exact,
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
            FieldIndex::SemanticIndex {} => topk_rs::proto::v1::control::FieldIndex::semantic(),
            FieldIndex::NGramIndex {} => topk_rs::proto::v1::control::FieldIndex::ngram(),
            FieldIndex::MultiVectorIndex {
                metric,
                quantization,
                width,
                top_k,
            } => topk_rs::proto::v1::control::FieldIndex::multi_vector(
                metric.into(),
                quantization.map(|q| q.into()),
                width,
                top_k,
            ),
            FieldIndex::Unknown() => panic!("cannot write an unknown field index: {UNSUPPORTED}"),
        }
    }
}

impl From<topk_rs::proto::v1::control::FieldIndex> for FieldIndex {
    fn from(proto: topk_rs::proto::v1::control::FieldIndex) -> Self {
        let index = match proto.index {
            Some(index) => index,
            None => return FieldIndex::Unknown(),
        };
        match index {
            topk_rs::proto::v1::control::field_index::Index::KeywordIndex(keyword_index) => {
                FieldIndex::KeywordIndex {
                    index_type: match keyword_index.index_type() {
                        topk_rs::proto::v1::control::KeywordIndexType::Text => {
                            KeywordIndexType::Text
                        }
                        topk_rs::proto::v1::control::KeywordIndexType::Exact => {
                            KeywordIndexType::Exact
                        }
                        _ => return FieldIndex::Unknown(),
                    },
                }
            }
            topk_rs::proto::v1::control::field_index::Index::VectorIndex(vector_index) => {
                FieldIndex::VectorIndex {
                    metric: match vector_index.metric() {
                        topk_rs::proto::v1::control::VectorDistanceMetric::Cosine => {
                            VectorDistanceMetric::Cosine
                        }
                        topk_rs::proto::v1::control::VectorDistanceMetric::Euclidean => {
                            VectorDistanceMetric::Euclidean
                        }
                        topk_rs::proto::v1::control::VectorDistanceMetric::DotProduct => {
                            VectorDistanceMetric::DotProduct
                        }
                        topk_rs::proto::v1::control::VectorDistanceMetric::Hamming => {
                            VectorDistanceMetric::Hamming
                        }
                        _ => return FieldIndex::Unknown(),
                    },
                }
            }
            topk_rs::proto::v1::control::field_index::Index::SemanticIndex(_) => {
                FieldIndex::SemanticIndex {}
            }
            topk_rs::proto::v1::control::field_index::Index::NgramIndex(_) => {
                FieldIndex::NGramIndex {}
            }
            topk_rs::proto::v1::control::field_index::Index::MultiVectorIndex(mvi) => {
                FieldIndex::MultiVectorIndex {
                    metric: match mvi.metric() {
                        topk_rs::proto::v1::control::MultiVectorDistanceMetric::Maxsim => {
                            MultiVectorDistanceMetric::Maxsim
                        }
                        _ => return FieldIndex::Unknown(),
                    },
                    quantization: match mvi.quantization {
                        Some(q) => match MultiVectorQuantizationPb::try_from(q) {
                            Ok(MultiVectorQuantizationPb::Binary1bit) => {
                                Some(MultiVectorQuantization::Binary1bit)
                            }
                            Ok(MultiVectorQuantizationPb::Binary2bit) => {
                                Some(MultiVectorQuantization::Binary2bit)
                            }
                            Ok(MultiVectorQuantizationPb::Scalar) => {
                                Some(MultiVectorQuantization::Scalar)
                            }
                            _ => return FieldIndex::Unknown(),
                        },
                        // `None` means unset, not an unrecognised value
                        None => None,
                    },
                    width: mvi.width,
                    top_k: mvi.top_k,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use topk_rs::proto::v1::control as pb;

    #[rstest]
    #[case::outer_oneof(pb::FieldIndex { index: None })]
    #[case::vector_metric(pb::FieldIndex {
        index: Some(pb::field_index::Index::VectorIndex(pb::VectorIndex {
            metric: 9999,
            ..Default::default()
        })),
    })]
    #[case::quantization(pb::FieldIndex {
        index: Some(pb::field_index::Index::MultiVectorIndex(pb::MultiVectorIndex {
            metric: pb::MultiVectorDistanceMetric::Maxsim as i32,
            quantization: Some(9999),
            ..Default::default()
        })),
    })]
    fn unknown_field_index_decodes_to_unknown(#[case] proto: pb::FieldIndex) {
        assert_eq!(FieldIndex::from(proto), FieldIndex::Unknown());
    }
}
