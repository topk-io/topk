use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::data::UNSUPPORTED;
use topk_rs::proto::v1::control::{
    KeywordIndexType as KeywordIndexTypePb,
    MultiVectorDistanceMetric as MultiVectorDistanceMetricPb,
    MultiVectorQuantization as MultiVectorQuantizationPb,
    VectorDistanceMetric as VectorDistanceMetricPb,
};

/// @internal
/// @hideconstructor
#[napi(namespace = "schema")]
#[derive(Clone, Debug)]
pub struct FieldIndex(pub(crate) FieldIndexUnion);

/// @ignore
#[napi(namespace = "schema")]
#[derive(Clone, Debug)]
pub enum FieldIndexUnion {
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
    Unknown {},
}

impl FieldIndex {
    pub(crate) fn vector_index(metric: VectorDistanceMetric) -> Self {
        Self(FieldIndexUnion::VectorIndex { metric })
    }

    pub(crate) fn keyword_index(index_type: KeywordIndexType) -> Self {
        Self(FieldIndexUnion::KeywordIndex { index_type })
    }

    pub(crate) fn unknown() -> Self {
        Self(FieldIndexUnion::Unknown {})
    }

    pub(crate) fn semantic_index() -> Self {
        Self(FieldIndexUnion::SemanticIndex {})
    }

    pub(crate) fn ngram_index() -> Self {
        Self(FieldIndexUnion::NGramIndex {})
    }

    pub(crate) fn multi_vector_index(
        metric: MultiVectorDistanceMetric,
        quantization: Option<MultiVectorQuantization>,
        width: Option<u32>,
        top_k: Option<u32>,
    ) -> Self {
        Self(FieldIndexUnion::MultiVectorIndex {
            metric,
            quantization,
            width,
            top_k,
        })
    }
}

impl FromNapiValue for FieldIndex {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(index) = crate::try_cast_ref!(env, value, FieldIndex) {
            return Ok(index.clone());
        }

        Err(napi::Error::from_reason("Invalid field index"))
    }
}

#[napi(string_enum = "camelCase", namespace = "schema")]
#[derive(Clone, Debug)]
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

#[napi(string_enum = "snake_case", namespace = "schema")]
#[derive(Clone, Debug)]
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

#[napi(string_enum = "snake_case", namespace = "schema")]
#[derive(Clone, Debug)]
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

#[napi(string_enum, namespace = "schema")]
#[derive(Clone, Debug)]
pub enum MultiVectorQuantization {
    #[napi(value = "1bit")]
    Binary1bit,
    #[napi(value = "2bit")]
    Binary2bit,
    #[napi(value = "scalar")]
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

impl From<topk_rs::proto::v1::control::FieldIndex> for FieldIndexUnion {
    fn from(field_index: topk_rs::proto::v1::control::FieldIndex) -> Self {
        FieldIndex::from(field_index).0
    }
}

impl From<topk_rs::proto::v1::control::FieldIndex> for FieldIndex {
    fn from(field_index: topk_rs::proto::v1::control::FieldIndex) -> Self {
        match field_index.index {
            Some(i) => match i {
                topk_rs::proto::v1::control::field_index::Index::KeywordIndex(k) => {
                    match k.index_type() {
                        KeywordIndexTypePb::Text => {
                            FieldIndex::keyword_index(KeywordIndexType::Text)
                        }
                        KeywordIndexTypePb::Exact => {
                            FieldIndex::keyword_index(KeywordIndexType::Exact)
                        }
                        KeywordIndexTypePb::Unspecified => return FieldIndex::unknown(),
                    }
                }
                topk_rs::proto::v1::control::field_index::Index::VectorIndex(v) => {
                    match v.metric() {
                        VectorDistanceMetricPb::Cosine => {
                            FieldIndex::vector_index(VectorDistanceMetric::Cosine)
                        }
                        VectorDistanceMetricPb::Euclidean => {
                            FieldIndex::vector_index(VectorDistanceMetric::Euclidean)
                        }
                        VectorDistanceMetricPb::DotProduct => {
                            FieldIndex::vector_index(VectorDistanceMetric::DotProduct)
                        }
                        VectorDistanceMetricPb::Hamming => {
                            FieldIndex::vector_index(VectorDistanceMetric::Hamming)
                        }
                        VectorDistanceMetricPb::Unspecified => return FieldIndex::unknown(),
                    }
                }
                topk_rs::proto::v1::control::field_index::Index::SemanticIndex(_) => {
                    FieldIndex::semantic_index()
                }
                topk_rs::proto::v1::control::field_index::Index::NgramIndex(_) => {
                    FieldIndex::ngram_index()
                }
                topk_rs::proto::v1::control::field_index::Index::MultiVectorIndex(mvi) => {
                    let metric = match mvi.metric() {
                        MultiVectorDistanceMetricPb::Maxsim => MultiVectorDistanceMetric::Maxsim,
                        MultiVectorDistanceMetricPb::Unspecified => return FieldIndex::unknown(),
                    };
                    let quantization = match mvi.quantization {
                        None => None,
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
                            _ => return FieldIndex::unknown(),
                        },
                    };
                    FieldIndex::multi_vector_index(metric, quantization, mvi.width, mvi.top_k)
                }
            },
            None => FieldIndex::unknown(),
        }
    }
}

impl From<FieldIndex> for topk_rs::proto::v1::control::FieldIndex {
    fn from(field_index: FieldIndex) -> Self {
        match field_index.0 {
            FieldIndexUnion::KeywordIndex { index_type } => {
                topk_rs::proto::v1::control::FieldIndex::keyword(index_type.into())
            }
            FieldIndexUnion::VectorIndex { metric } => {
                topk_rs::proto::v1::control::FieldIndex::vector(metric.into())
            }
            FieldIndexUnion::SemanticIndex {} => {
                topk_rs::proto::v1::control::FieldIndex::semantic()
            }
            FieldIndexUnion::NGramIndex {} => topk_rs::proto::v1::control::FieldIndex::ngram(),
            FieldIndexUnion::MultiVectorIndex {
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
            FieldIndexUnion::Unknown {} => {
                panic!("cannot write an unknown field index: {UNSUPPORTED}")
            }
        }
    }
}
