use super::*;

impl FieldIndex {
    pub fn keyword(index_type: KeywordIndexType) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::KeywordIndex(KeywordIndex {
                index_type: index_type as i32,
            })),
        }
    }

    pub fn vector(metric: VectorDistanceMetric) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::VectorIndex(VectorIndex {
                metric: metric as i32,
            })),
        }
    }
}
