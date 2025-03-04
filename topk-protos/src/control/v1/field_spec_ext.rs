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

    pub fn integer(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::Integer(FieldTypeInteger {})),
            }),
            required,
            index: None,
        }
    }

    pub fn float(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::Float(FieldTypeFloat {})),
            }),
            required,
            index: None,
        }
    }

    pub fn boolean(required: bool) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::Boolean(FieldTypeBoolean {})),
            }),
            required,
            index: None,
        }
    }

    pub fn f32_vector(dimension: u32, required: bool, metric: VectorDistanceMetric) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::f32_vector(dimension)),
            }),
            required,
            index: Some(FieldIndex {
                index: Some(field_index::Index::VectorIndex(VectorIndex {
                    metric: metric as i32,
                })),
            }),
        }
    }

    pub fn u8_vector(dimension: u32, required: bool, metric: VectorDistanceMetric) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::u8_vector(dimension)),
            }),
            required,
            index: Some(FieldIndex {
                index: Some(field_index::Index::VectorIndex(VectorIndex {
                    metric: metric as i32,
                })),
            }),
        }
    }

    pub fn binary_vector(
        dimension: u32,
        required: bool,
        metric: VectorDistanceMetric,
    ) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::binary_vector(dimension)),
            }),
            required,
            index: Some(FieldIndex {
                index: Some(field_index::Index::VectorIndex(VectorIndex {
                    metric: metric as i32,
                })),
            }),
        }
    }

    pub fn semantic(
        required: bool,
        model: Option<String>,
        embedding_type: Option<EmbeddingDataType>,
    ) -> FieldSpec {
        FieldSpec {
            data_type: Some(FieldType {
                data_type: Some(field_type::DataType::Text(FieldTypeText {})),
            }),
            required,
            index: Some(FieldIndex {
                index: Some(field_index::Index::SemanticIndex(SemanticIndex {
                    model,
                    embedding_type: embedding_type.map(|dt| dt.into()),
                })),
            }),
        }
    }
}
