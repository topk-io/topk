use super::*;

impl FieldSpec {
    pub fn text(required: bool, index_type: Option<KeywordIndexType>) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::Text(FieldTypeText {})),
            }),
            required,
            index: index_type.map(|index_type| FieldIndex {
                index: Some(field_index::Index::KeywordIndex(KeywordIndex {
                    index_type: index_type as i32,
                })),
            }),
        }
    }

    pub fn float_vector(dimension: u32, required: bool, metric: VectorDistanceMetric) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::FloatVector(FieldTypeFloatVector {
                    dimension,
                })),
            }),
            required,
            index: Some(FieldIndex {
                index: Some(field_index::Index::VectorIndex(VectorIndex {
                    metric: metric as i32,
                })),
            }),
        }
    }

    pub fn integer(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::Integer(FieldTypeInteger {})),
            }),
            required,
            index: None,
        }
    }
}
