use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(namespace = "schema")]
#[derive(Clone, Debug)]
pub struct FieldIndex(pub(crate) FieldIndexUnion);

#[napi(namespace = "schema")]
#[derive(Clone, Debug)]
pub enum FieldIndexUnion {
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

impl FieldIndex {
    pub(crate) fn vector_index(metric: VectorDistanceMetric) -> Self {
        Self(FieldIndexUnion::VectorIndex { metric })
    }

    pub(crate) fn keyword_index(index_type: KeywordIndexType) -> Self {
        Self(FieldIndexUnion::KeywordIndex { index_type })
    }

    pub(crate) fn semantic_index(
        model: Option<String>,
        embedding_type: Option<EmbeddingDataType>,
    ) -> Self {
        Self(FieldIndexUnion::SemanticIndex {
            model,
            embedding_type,
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
}

impl From<KeywordIndexType> for topk_rs::proto::v1::control::KeywordIndexType {
    fn from(index_type: KeywordIndexType) -> Self {
        match index_type {
            KeywordIndexType::Text => topk_rs::proto::v1::control::KeywordIndexType::Text,
        }
    }
}

impl From<topk_rs::proto::v1::control::KeywordIndexType> for KeywordIndexType {
    fn from(index_type: topk_rs::proto::v1::control::KeywordIndexType) -> Self {
        match index_type {
            topk_rs::proto::v1::control::KeywordIndexType::Text => KeywordIndexType::Text,
            topk_rs::proto::v1::control::KeywordIndexType::Unspecified => {
                unreachable!("Invalid proto: Unspecified keyword index type")
            }
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

impl From<topk_rs::proto::v1::control::VectorDistanceMetric> for VectorDistanceMetric {
    fn from(metric: topk_rs::proto::v1::control::VectorDistanceMetric) -> Self {
        match metric {
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
            topk_rs::proto::v1::control::VectorDistanceMetric::Unspecified => {
                unreachable!("Invalid proto: Unspecified vector distance metric")
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

#[napi(string_enum = "lowercase", namespace = "schema")]
#[derive(Clone, Debug)]
pub enum EmbeddingDataType {
    Float32,
    UInt8,
    Binary,
}

impl From<topk_rs::proto::v1::control::EmbeddingDataType> for EmbeddingDataType {
    fn from(embedding_type: topk_rs::proto::v1::control::EmbeddingDataType) -> Self {
        match embedding_type {
            topk_rs::proto::v1::control::EmbeddingDataType::F32 => EmbeddingDataType::Float32,
            topk_rs::proto::v1::control::EmbeddingDataType::U8 => EmbeddingDataType::UInt8,
            topk_rs::proto::v1::control::EmbeddingDataType::Binary => EmbeddingDataType::Binary,
            topk_rs::proto::v1::control::EmbeddingDataType::Unspecified => {
                unreachable!("Invalid proto: Unspecified embedding data type")
            }
        }
    }
}

impl From<EmbeddingDataType> for topk_rs::proto::v1::control::EmbeddingDataType {
    fn from(embedding_type: EmbeddingDataType) -> Self {
        match embedding_type {
            EmbeddingDataType::Float32 => topk_rs::proto::v1::control::EmbeddingDataType::F32,
            EmbeddingDataType::UInt8 => topk_rs::proto::v1::control::EmbeddingDataType::U8,
            EmbeddingDataType::Binary => topk_rs::proto::v1::control::EmbeddingDataType::Binary,
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
                    FieldIndex::keyword_index(
                        topk_rs::proto::v1::control::KeywordIndexType::try_from(k.index_type)
                            .expect("Unsupported keyword index type")
                            .into(),
                    )
                }
                topk_rs::proto::v1::control::field_index::Index::VectorIndex(v) => {
                    FieldIndex::vector_index(
                        topk_rs::proto::v1::control::VectorDistanceMetric::try_from(v.metric)
                            .expect("Unsupported vector distance metric")
                            .into(),
                    )
                }
                topk_rs::proto::v1::control::field_index::Index::SemanticIndex(s) => {
                    FieldIndex::semantic_index(
                        s.model,
                        s.embedding_type.map(|t| {
                            topk_rs::proto::v1::control::EmbeddingDataType::try_from(t)
                                .expect("Unsupported embedding data type")
                                .into()
                        }),
                    )
                }
            },
            None => unreachable!("Invalid field index proto"),
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
            FieldIndexUnion::SemanticIndex {
                model,
                embedding_type,
            } => topk_rs::proto::v1::control::FieldIndex::semantic(
                model,
                embedding_type.map(|t| t.into()),
            ),
        }
    }
}
