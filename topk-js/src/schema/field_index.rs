use napi::bindgen_prelude::*;
use napi_derive::napi;

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

    pub(crate) fn multi_vector_index(
        metric: MultiVectorDistanceMetric,
        sketch_bits: Option<u32>,
        quantization: Option<MultiVectorQuantization>,
    ) -> Self {
        Self(FieldIndexUnion::MultiVectorIndex {
            metric,
            sketch_bits,
            quantization,
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

impl TryFrom<topk_rs::proto::v1::control::KeywordIndexType> for KeywordIndexType {
    type Error = crate::error::TopkError;

    fn try_from(
        index_type: topk_rs::proto::v1::control::KeywordIndexType,
    ) -> std::result::Result<Self, Self::Error> {
        match index_type {
            topk_rs::proto::v1::control::KeywordIndexType::Text => Ok(KeywordIndexType::Text),
            topk_rs::proto::v1::control::KeywordIndexType::Unspecified => {
                Err(topk_rs::Error::InvalidProto.into())
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

impl TryFrom<topk_rs::proto::v1::control::VectorDistanceMetric> for VectorDistanceMetric {
    type Error = crate::error::TopkError;

    fn try_from(
        metric: topk_rs::proto::v1::control::VectorDistanceMetric,
    ) -> std::result::Result<Self, Self::Error> {
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

#[napi(string_enum = "lowercase", namespace = "schema")]
#[derive(Clone, Debug)]
pub enum EmbeddingDataType {
    Float32,
    UInt8,
    Binary,
}

impl TryFrom<topk_rs::proto::v1::control::EmbeddingDataType> for EmbeddingDataType {
    type Error = crate::error::TopkError;

    fn try_from(
        embedding_type: topk_rs::proto::v1::control::EmbeddingDataType,
    ) -> std::result::Result<Self, Self::Error> {
        match embedding_type {
            topk_rs::proto::v1::control::EmbeddingDataType::F32 => Ok(EmbeddingDataType::Float32),
            topk_rs::proto::v1::control::EmbeddingDataType::U8 => Ok(EmbeddingDataType::UInt8),
            topk_rs::proto::v1::control::EmbeddingDataType::Binary => {
                Ok(EmbeddingDataType::Binary)
            }
            topk_rs::proto::v1::control::EmbeddingDataType::Unspecified => {
                Err(topk_rs::Error::InvalidProto.into())
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

impl TryFrom<topk_rs::proto::v1::control::MultiVectorDistanceMetric> for MultiVectorDistanceMetric {
    type Error = crate::error::TopkError;

    fn try_from(
        metric: topk_rs::proto::v1::control::MultiVectorDistanceMetric,
    ) -> std::result::Result<Self, Self::Error> {
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

impl TryFrom<topk_rs::proto::v1::control::MultiVectorQuantization> for MultiVectorQuantization {
    type Error = crate::error::TopkError;

    fn try_from(
        quantization: topk_rs::proto::v1::control::MultiVectorQuantization,
    ) -> std::result::Result<Self, Self::Error> {
        match quantization {
            topk_rs::proto::v1::control::MultiVectorQuantization::Binary1bit => {
                Ok(MultiVectorQuantization::Binary1bit)
            }
            topk_rs::proto::v1::control::MultiVectorQuantization::Binary2bit => {
                Ok(MultiVectorQuantization::Binary2bit)
            }
            topk_rs::proto::v1::control::MultiVectorQuantization::Scalar => {
                Ok(MultiVectorQuantization::Scalar)
            }
            topk_rs::proto::v1::control::MultiVectorQuantization::Unspecified => {
                Err(topk_rs::Error::InvalidProto.into())
            }
        }
    }
}

impl TryFrom<topk_rs::proto::v1::control::FieldIndex> for FieldIndexUnion {
    type Error = crate::error::TopkError;

    fn try_from(
        field_index: topk_rs::proto::v1::control::FieldIndex,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(FieldIndex::try_from(field_index)?.0)
    }
}

impl TryFrom<topk_rs::proto::v1::control::FieldIndex> for FieldIndex {
    type Error = crate::error::TopkError;

    fn try_from(
        field_index: topk_rs::proto::v1::control::FieldIndex,
    ) -> std::result::Result<Self, Self::Error> {
        let index = field_index
            .index()
            .map_err(|_| topk_rs::Error::InvalidProto)?;
        Ok(match index {
            topk_rs::proto::v1::control::field_index::Index::KeywordIndex(k) => {
                FieldIndex::keyword_index(
                    topk_rs::proto::v1::control::KeywordIndexType::try_from(k.index_type)
                        .map_err(|_| topk_rs::Error::InvalidProto)?
                        .try_into()?,
                )
            }
            topk_rs::proto::v1::control::field_index::Index::VectorIndex(v) => {
                FieldIndex::vector_index(
                    topk_rs::proto::v1::control::VectorDistanceMetric::try_from(v.metric)
                        .map_err(|_| topk_rs::Error::InvalidProto)?
                        .try_into()?,
                )
            }
            topk_rs::proto::v1::control::field_index::Index::SemanticIndex(s) => {
                FieldIndex::semantic_index(
                    s.model.clone(),
                    s.embedding_type
                        .map(|t| {
                            topk_rs::proto::v1::control::EmbeddingDataType::try_from(t)
                                .map_err(|_| topk_rs::Error::InvalidProto.into())
                                .and_then(|p| p.try_into())
                        })
                        .transpose()?,
                )
            }
            topk_rs::proto::v1::control::field_index::Index::MultiVectorIndex(mvi) => {
                FieldIndex::multi_vector_index(
                    topk_rs::proto::v1::control::MultiVectorDistanceMetric::try_from(mvi.metric)
                        .map_err(|_| topk_rs::Error::InvalidProto)?
                        .try_into()?,
                    mvi.sketch_bits,
                    mvi.quantization
                        .map(|q| {
                            topk_rs::proto::v1::control::MultiVectorQuantization::try_from(q)
                                .map_err(|_| topk_rs::Error::InvalidProto.into())
                                .and_then(|p| p.try_into())
                        })
                        .transpose()?,
                )
            }
        })
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
            FieldIndexUnion::MultiVectorIndex {
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
