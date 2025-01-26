use pyo3::prelude::*;

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq)]
pub enum FieldIndex {
    KeywordIndex { index_type: KeywordIndexType },
    VectorIndex { metric: VectorDistanceMetric },
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum VectorDistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum KeywordIndexType {
    Unspecified,
    Text,
}

impl From<KeywordIndexType> for topk_protos::v1::control::KeywordIndexType {
    fn from(index_type: KeywordIndexType) -> Self {
        match index_type {
            KeywordIndexType::Unspecified => {
                topk_protos::v1::control::KeywordIndexType::Unspecified
            }
            KeywordIndexType::Text => topk_protos::v1::control::KeywordIndexType::Text,
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
        }
    }
}

impl From<topk_protos::v1::control::FieldIndex> for FieldIndex {
    fn from(proto: topk_protos::v1::control::FieldIndex) -> Self {
        match proto.index.expect("index is required") {
            topk_protos::v1::control::field_index::Index::KeywordIndex(keyword_index) => {
                FieldIndex::KeywordIndex {
                    index_type: match keyword_index.index_type {
                        index_type
                            if index_type
                                == topk_protos::v1::control::KeywordIndexType::Unspecified
                                    as i32 =>
                        {
                            KeywordIndexType::Unspecified
                        }
                        index_type
                            if index_type
                                == topk_protos::v1::control::KeywordIndexType::Text as i32 =>
                        {
                            KeywordIndexType::Text
                        }
                        _ => unreachable!("invalid index type {:?}", keyword_index.index_type),
                    },
                }
            }
            topk_protos::v1::control::field_index::Index::VectorIndex(_) => {
                FieldIndex::VectorIndex {
                    metric: match proto.index.expect("index is required") {
                        topk_protos::v1::control::field_index::Index::VectorIndex(index) => {
                            match index.metric {
                                metric
                                    if metric
                                        == topk_protos::v1::control::VectorDistanceMetric::Cosine
                                            as i32 =>
                                {
                                    VectorDistanceMetric::Cosine
                                }
                                metric
                                    if metric
                                        == topk_protos::v1::control::VectorDistanceMetric::Euclidean
                                            as i32 =>
                                {
                                    VectorDistanceMetric::Euclidean
                                }
                                _ => unreachable!("invalid metric {:?}", index.metric),
                            }
                        }
                        topk_protos::v1::control::field_index::Index::KeywordIndex(index) => {
                            match index.index_type {
                                index_type
                                    if index_type
                                        == topk_protos::v1::control::KeywordIndexType::Unspecified
                                            as i32 =>
                                {
                                    VectorDistanceMetric::Cosine
                                }
                                _ => unreachable!("invalid index type {:?}", index.index_type),
                            }
                        }
                    },
                }
            }
        }
    }
}
