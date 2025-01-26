use doc_validation::ValidationErrorBag;

use super::*;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchemaValidationError {
    #[error("field `{field}` has no data type")]
    MissingDataType { field: String },

    #[error("field name `{field}` cannot start with an underscore")]
    ReservedFieldName { field: String },

    #[error("invalid index `{index}` for field type `{data_type}`")]
    InvalidIndex {
        field: String,
        index: String,
        data_type: String,
    },

    #[error("vector field `{field}` cannot be nullable")]
    VectorFieldCannotBeNullable { field: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub struct IndexSchema {
    schema: HashMap<String, FieldSpec>,
}

impl IndexSchema {
    pub fn new(schema: HashMap<String, FieldSpec>) -> Self {
        Self { schema }
    }

    pub fn fields(&self) -> &HashMap<String, FieldSpec> {
        &self.schema
    }

    pub fn into_fields(self) -> HashMap<String, FieldSpec> {
        self.schema
    }
}

impl IntoIterator for IndexSchema {
    type Item = (String, FieldSpec);
    type IntoIter = std::collections::hash_map::IntoIter<String, FieldSpec>;

    fn into_iter(self) -> Self::IntoIter {
        self.schema.into_iter()
    }
}

impl<const N: usize, T> TryFrom<[(T, FieldSpec); N]> for IndexSchema
where
    T: Into<String>,
{
    type Error = ValidationErrorBag<SchemaValidationError>;

    fn try_from(entries: [(T, FieldSpec); N]) -> Result<Self, Self::Error> {
        let entries: HashMap<String, FieldSpec> = entries
            .into_iter()
            .map(|(field, spec)| (field.into(), spec.clone()))
            .collect();

        IndexSchema::try_from(entries)
    }
}

impl TryFrom<HashMap<String, FieldSpec>> for IndexSchema {
    type Error = ValidationErrorBag<SchemaValidationError>;

    fn try_from(entries: HashMap<String, FieldSpec>) -> Result<Self, Self::Error> {
        let mut errors: ValidationErrorBag<SchemaValidationError> = ValidationErrorBag::new(vec![]);

        for (field, spec) in entries.iter() {
            errors.extend(validate_field_spec(field, spec));
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(IndexSchema {
            schema: entries
                .iter()
                .map(|(field, spec)| (field.clone(), spec.clone()))
                .collect(),
        })
    }
}

fn validate_field_spec(field: &str, spec: &FieldSpec) -> Vec<SchemaValidationError> {
    let mut errors = vec![];

    if field.starts_with("_") {
        errors.push(SchemaValidationError::ReservedFieldName {
            field: field.to_string(),
        });
    }

    let data_type = spec.data_type.map(|t| t.data_type).flatten();

    // if field has an index, we have to check that it is valid
    // rules:
    // - `keyword index` can only be used with `text` data type
    // - `vector index` can only be used with `float_vector` and `byte_vector` data types
    if let Some(index) = spec.index.map(|i| i.index).flatten() {
        match index {
            field_index::Index::KeywordIndex(_) => match data_type {
                Some(field_type::DataType::Text(..)) => {}
                Some(data_type) => {
                    errors.push(SchemaValidationError::InvalidIndex {
                        field: field.to_string(),
                        index: "keyword".to_string(),
                        data_type: data_type.to_user_friendly_type_name(),
                    });
                }
                None => {
                    errors.push(SchemaValidationError::MissingDataType {
                        field: field.to_string(),
                    });
                }
            },
            field_index::Index::VectorIndex(_) => match data_type {
                Some(field_type::DataType::FloatVector(..))
                | Some(field_type::DataType::ByteVector(..)) => {
                    if !spec.required {
                        errors.push(SchemaValidationError::VectorFieldCannotBeNullable {
                            field: field.to_string(),
                        });
                    }
                }
                Some(data_type) => {
                    errors.push(SchemaValidationError::InvalidIndex {
                        field: field.to_string(),
                        index: "vector".to_string(),
                        data_type: data_type.to_user_friendly_type_name(),
                    });
                }
                None => {
                    errors.push(SchemaValidationError::MissingDataType {
                        field: field.to_string(),
                    });
                }
            },
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1::control::{
        field_type::DataType, FieldIndex, FieldType, FieldTypeInteger, FieldTypeText, KeywordIndex,
        KeywordIndexType, VectorDistanceMetric, VectorIndex,
    };

    #[test]
    fn test_validate_schema_id_field() {
        let errors = IndexSchema::try_from([(
            "_id",
            FieldSpec {
                data_type: Some(FieldType {
                    data_type: Some(DataType::Text(FieldTypeText {})),
                }),
                required: true,
                index: None,
            },
        )])
        .expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([SchemaValidationError::ReservedFieldName {
                field: "_id".to_string()
            }])
        );
    }

    #[test]
    fn test_validate_schema_reserved_field_name() {
        let errors = IndexSchema::try_from([(
            "_reserved",
            FieldSpec {
                data_type: Some(FieldType {
                    data_type: Some(DataType::Text(FieldTypeText {})),
                }),
                required: true,
                index: None,
            },
        )])
        .expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([SchemaValidationError::ReservedFieldName {
                field: "_reserved".to_string()
            }])
        );
    }

    #[test]
    fn test_validate_schema_keyword_index_validation() {
        assert!(IndexSchema::try_from([(
            "int_field",
            FieldSpec {
                data_type: Some(FieldType {
                    data_type: Some(DataType::Text(FieldTypeText {})),
                }),
                required: false,
                index: Some(FieldIndex {
                    index: Some(field_index::Index::KeywordIndex(KeywordIndex {
                        index_type: KeywordIndexType::Text as i32,
                    })),
                }),
            }
        )])
        .is_ok());

        assert_eq!(
            IndexSchema::try_from([(
                "int_field",
                FieldSpec {
                    data_type: Some(FieldType {
                        data_type: Some(DataType::Integer(FieldTypeInteger {})),
                    }),
                    required: false,
                    index: Some(FieldIndex {
                        index: Some(field_index::Index::KeywordIndex(KeywordIndex {
                            index_type: KeywordIndexType::Text as i32,
                        })),
                    }),
                }
            )])
            .unwrap_err(),
            ValidationErrorBag::from([SchemaValidationError::InvalidIndex {
                field: "int_field".to_string(),
                index: "keyword".to_string(),
                data_type: "integer".to_string(),
            }])
        );
    }

    #[test]
    fn test_validate_schema_vector_index_validation() {
        assert!(IndexSchema::try_from([(
            "vector_field",
            FieldSpec {
                data_type: Some(FieldType {
                    data_type: Some(DataType::FloatVector(FieldTypeFloatVector {
                        dimension: 512
                    })),
                }),
                required: true,
                index: Some(FieldIndex {
                    index: Some(field_index::Index::VectorIndex(VectorIndex {
                        metric: VectorDistanceMetric::Cosine as i32,
                    })),
                }),
            }
        )])
        .is_ok());

        assert!(IndexSchema::try_from([(
            "vector_field",
            FieldSpec {
                data_type: Some(FieldType {
                    data_type: Some(DataType::ByteVector(FieldTypeByteVector { dimension: 512 })),
                }),
                required: true,
                index: Some(FieldIndex {
                    index: Some(field_index::Index::VectorIndex(VectorIndex {
                        metric: VectorDistanceMetric::Cosine as i32,
                    })),
                }),
            }
        )])
        .is_ok());

        assert_eq!(
            IndexSchema::try_from([(
                "vec_field",
                FieldSpec {
                    data_type: Some(FieldType {
                        data_type: Some(DataType::FloatVector(FieldTypeFloatVector {
                            dimension: 512
                        })),
                    }),
                    required: false,
                    index: Some(FieldIndex {
                        index: Some(field_index::Index::VectorIndex(VectorIndex {
                            metric: VectorDistanceMetric::Cosine as i32,
                        })),
                    }),
                }
            )])
            .unwrap_err(),
            ValidationErrorBag::from([SchemaValidationError::VectorFieldCannotBeNullable {
                field: "vec_field".to_string()
            }])
        );

        assert_eq!(
            IndexSchema::try_from([(
                "int_field",
                FieldSpec {
                    data_type: Some(FieldType {
                        data_type: Some(DataType::Integer(FieldTypeInteger {})),
                    }),
                    required: true,
                    index: Some(FieldIndex {
                        index: Some(field_index::Index::VectorIndex(VectorIndex {
                            metric: VectorDistanceMetric::Cosine as i32,
                        })),
                    }),
                }
            )])
            .unwrap_err(),
            ValidationErrorBag::from([SchemaValidationError::InvalidIndex {
                field: "int_field".to_string(),
                index: "vector".to_string(),
                data_type: "integer".to_string(),
            }]),
        );
    }
}

impl field_type::DataType {
    pub fn to_user_friendly_type_name(&self) -> String {
        match self {
            field_type::DataType::Text(..) => "text".to_string(),
            field_type::DataType::Integer(..) => "integer".to_string(),
            field_type::DataType::Float(..) => "float".to_string(),
            field_type::DataType::Boolean(..) => "boolean".to_string(),
            field_type::DataType::FloatVector(..) => "float_vector".to_string(),
            field_type::DataType::ByteVector(..) => "byte_vector".to_string(),
            field_type::DataType::Bytes(..) => "bytes".to_string(),
        }
    }
}
