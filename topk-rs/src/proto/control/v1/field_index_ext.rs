use super::*;

impl FieldIndex {
    pub fn keyword(index_type: KeywordIndexType) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::KeywordIndex(KeywordIndex {
                index_type: index_type.into(),
            })),
        }
    }

    pub fn vector(metric: VectorDistanceMetric) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::VectorIndex(VectorIndex {
                metric: metric.into(),
                exact: None,
            })),
        }
    }

    pub fn multi_vector(
        metric: MultiVectorDistanceMetric,
        sketch_bits: Option<u32>,
        quantization: Option<MultiVectorQuantization>,
    ) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::MultiVectorIndex(MultiVectorIndex {
                metric: metric.into(),
                sketch_bits,
                quantization: quantization.map(|q| q.into()),
            })),
        }
    }

    pub fn semantic(
        model: Option<String>,
        embedding_type: Option<EmbeddingDataType>,
    ) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::SemanticIndex(SemanticIndex {
                model,
                embedding_type: embedding_type.map(|dt| dt.into()),
            })),
        }
    }
}
