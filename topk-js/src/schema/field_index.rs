use napi_derive::napi;

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

#[napi(object, namespace = "schema")]
#[derive(Clone, Debug)]
pub struct FieldIndex {
    pub index: Option<FieldIndexUnion>,
}

#[napi(string_enum = "camelCase", namespace = "schema")]
#[derive(Clone, Debug)]
pub enum KeywordIndexType {
    Text,
}

impl From<KeywordIndexType> for topk_protos::v1::control::KeywordIndexType {
    fn from(index_type: KeywordIndexType) -> Self {
        match index_type {
            KeywordIndexType::Text => topk_protos::v1::control::KeywordIndexType::Text.into(),
        }
    }
}

impl From<topk_protos::v1::control::KeywordIndexType> for KeywordIndexType {
    fn from(index_type: topk_protos::v1::control::KeywordIndexType) -> Self {
        match index_type {
            topk_protos::v1::control::KeywordIndexType::Text => KeywordIndexType::Text,
            topk_protos::v1::control::KeywordIndexType::Unspecified => {
                unreachable!("Unspecified keyword index type")
            }
        }
    }
}

impl From<topk_protos::v1::control::FieldIndex> for FieldIndexUnion {
    fn from(field_index: topk_protos::v1::control::FieldIndex) -> Self {
        match field_index.index {
            Some(i) => match i {
                topk_protos::v1::control::field_index::Index::KeywordIndex(k) => {
                    FieldIndexUnion::KeywordIndex {
                        index_type: topk_protos::v1::control::KeywordIndexType::try_from(
                            k.index_type,
                        )
                        .expect("Unsupported keyword index type")
                        .into(),
                    }
                }
                topk_protos::v1::control::field_index::Index::VectorIndex(v) => {
                    FieldIndexUnion::VectorIndex {
                        metric: topk_protos::v1::control::VectorDistanceMetric::try_from(v.metric)
                            .expect("Unsupported vector distance metric")
                            .into(),
                    }
                }
                topk_protos::v1::control::field_index::Index::SemanticIndex(s) => {
                    FieldIndexUnion::SemanticIndex {
                        model: s.model,
                        embedding_type: s.embedding_type.map(|t| {
                            topk_protos::v1::control::EmbeddingDataType::try_from(t)
                                .expect("Unsupported embedding data type")
                                .into()
                        }),
                    }
                }
            },
            None => unreachable!("Field index cannot be none"),
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

impl From<topk_protos::v1::control::VectorDistanceMetric> for VectorDistanceMetric {
    fn from(metric: topk_protos::v1::control::VectorDistanceMetric) -> Self {
        match metric {
            topk_protos::v1::control::VectorDistanceMetric::Cosine => VectorDistanceMetric::Cosine,
            topk_protos::v1::control::VectorDistanceMetric::Euclidean => {
                VectorDistanceMetric::Euclidean
            }
            topk_protos::v1::control::VectorDistanceMetric::DotProduct => {
                VectorDistanceMetric::DotProduct
            }
            topk_protos::v1::control::VectorDistanceMetric::Hamming => {
                VectorDistanceMetric::Hamming
            }
            topk_protos::v1::control::VectorDistanceMetric::Unspecified => {
                unreachable!("Unspecified vector distance metric")
            }
        }
    }
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

#[napi(string_enum = "lowercase", namespace = "schema")]
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
            EmbeddingDataType::Float32 => topk_protos::v1::control::EmbeddingDataType::F32,
            EmbeddingDataType::UInt8 => topk_protos::v1::control::EmbeddingDataType::U8,
            EmbeddingDataType::Binary => topk_protos::v1::control::EmbeddingDataType::Binary,
        }
    }
}

impl From<EmbeddingDataType> for Option<topk_protos::v1::control::EmbeddingDataType> {
    fn from(embedding_type: EmbeddingDataType) -> Self {
        Some(match embedding_type {
            EmbeddingDataType::Float32 => topk_protos::v1::control::EmbeddingDataType::F32,
            EmbeddingDataType::UInt8 => topk_protos::v1::control::EmbeddingDataType::U8,
            EmbeddingDataType::Binary => topk_protos::v1::control::EmbeddingDataType::Binary,
        })
    }
}

impl From<FieldIndexUnion> for topk_protos::v1::control::FieldIndex {
    fn from(field_index: FieldIndexUnion) -> Self {
        match field_index {
            FieldIndexUnion::KeywordIndex { index_type } => {
                topk_protos::v1::control::FieldIndex::keyword(index_type.into())
            }
            FieldIndexUnion::VectorIndex { metric } => {
                topk_protos::v1::control::FieldIndex::vector(metric.into())
            }
            FieldIndexUnion::SemanticIndex {
                model,
                embedding_type,
            } => topk_protos::v1::control::FieldIndex::semantic(
                model,
                embedding_type.map(|t| t.into()).unwrap_or_default(),
            ),
        }
    }
}

impl From<topk_protos::v1::control::FieldIndex> for FieldIndex {
    fn from(field_index: topk_protos::v1::control::FieldIndex) -> Self {
        match field_index.index {
            Some(i) => match i {
                topk_protos::v1::control::field_index::Index::KeywordIndex(k) => FieldIndex {
                    index: Some(FieldIndexUnion::KeywordIndex {
                        index_type: topk_protos::v1::control::KeywordIndexType::try_from(
                            k.index_type,
                        )
                        .expect("Unsupported keyword index type")
                        .into(),
                    }),
                },
                topk_protos::v1::control::field_index::Index::VectorIndex(v) => FieldIndex {
                    index: Some(FieldIndexUnion::VectorIndex {
                        metric: topk_protos::v1::control::VectorDistanceMetric::try_from(v.metric)
                            .expect("Unsupported vector distance metric")
                            .into(),
                    }),
                },
                topk_protos::v1::control::field_index::Index::SemanticIndex(s) => FieldIndex {
                    index: Some(FieldIndexUnion::SemanticIndex {
                        model: s.model,
                        embedding_type: s.embedding_type.map(|t| {
                            topk_protos::v1::control::EmbeddingDataType::try_from(t)
                                .expect("Unsupported embedding data type")
                                .into()
                        }),
                    }),
                },
            },
            None => FieldIndex { index: None },
        }
    }
}

impl From<FieldIndex> for topk_protos::v1::control::FieldIndex {
    fn from(field_index: FieldIndex) -> Self {
        match field_index.index {
            Some(FieldIndexUnion::KeywordIndex { index_type }) => {
                topk_protos::v1::control::FieldIndex::keyword(
                    topk_protos::v1::control::KeywordIndexType::from(index_type),
                )
            }
            Some(FieldIndexUnion::VectorIndex { metric }) => {
                topk_protos::v1::control::FieldIndex::vector(
                    topk_protos::v1::control::VectorDistanceMetric::from(metric),
                )
            }
            Some(FieldIndexUnion::SemanticIndex {
                model,
                embedding_type,
            }) => topk_protos::v1::control::FieldIndex::semantic(
                model,
                embedding_type.map(topk_protos::v1::control::EmbeddingDataType::from),
            ),
            None => topk_protos::v1::control::FieldIndex { index: None },
        }
    }
}
