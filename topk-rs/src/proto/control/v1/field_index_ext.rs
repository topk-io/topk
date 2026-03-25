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
        quantization: Option<MultiVectorQuantization>,
        width: Option<u32>,
        top_k: Option<u32>,
    ) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::MultiVectorIndex(MultiVectorIndex {
                metric: metric.into(),
                #[allow(deprecated)]
                sketch_bits: None,
                quantization: quantization.map(|q| q.into()),
                width,
                top_k,
                skip_smve: false,
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

    /// Skip sparse multi-vector encoding (SMVE) for multi-vector index.
    pub fn skip_smve(mut self) -> Self {
        if let Some(field_index::Index::MultiVectorIndex(mvi)) = self.index.as_mut() {
            mvi.skip_smve = true;
        }
        self
    }
}
