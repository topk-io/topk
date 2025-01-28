use super::*;
use crate::v1::data;
use collection_schema::CollectionSchema;
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ValidationErrorBag<T: Serialize>(Vec<T>);

impl<T: Serialize + DeserializeOwned> ValidationErrorBag<T> {
    pub fn new(errors: Vec<T>) -> Self {
        Self(errors)
    }

    pub fn from(errors: impl IntoIterator<Item = T>) -> Self {
        Self(errors.into_iter().collect())
    }

    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn push(&mut self, error: T) {
        self.0.push(error);
    }

    pub fn extend(&mut self, errors: impl IntoIterator<Item = T>) {
        self.0.extend(errors.into_iter());
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ValidationError {
    MissingId {
        doc_offset: usize,
    },

    InvalidId {
        doc_offset: usize,
        got: String,
    },

    MissingField {
        doc_id: String,
        field: String,
    },

    ReservedFieldName {
        doc_id: String,
        field: String,
    },

    InvalidDataType {
        doc_id: String,
        field: String,
        expected_type: String,
        got_value: String,
    },

    NoDocuments,
}

impl<T: Serialize> From<ValidationErrorBag<T>> for tonic::Status {
    fn from(err: ValidationErrorBag<T>) -> Self {
        match serde_json::to_string(&err) {
            Ok(message) => tonic::Status::invalid_argument(message),
            Err(e) => {
                error!(?e, "failed to serialize response");
                tonic::Status::internal("failed to serialize response")
            }
        }
    }
}

impl<T: Serialize + DeserializeOwned> TryFrom<tonic::Status> for ValidationErrorBag<T> {
    type Error = serde_json::Error;

    fn try_from(status: tonic::Status) -> Result<Self, Self::Error> {
        serde_json::from_str(status.message())
    }
}

pub fn validate_documents(
    schema: &CollectionSchema,
    documents: &[data::Document],
) -> Result<(), ValidationErrorBag<ValidationError>> {
    let mut error_bag = ValidationErrorBag::empty();

    if documents.is_empty() {
        return Err(ValidationErrorBag::from([ValidationError::NoDocuments]));
    }

    for (i, doc) in documents.iter().enumerate() {
        // Validate `_id` field
        let id = match doc.fields.get("_id") {
            Some(id) => match &id.value {
                Some(data::value::Value::String(id)) if !id.is_empty() => id,
                _ => {
                    error_bag.push(ValidationError::InvalidId {
                        doc_offset: i,
                        got: format!("{:?}", id),
                    });
                    continue;
                }
            },
            None => {
                error_bag.push(ValidationError::MissingId { doc_offset: i });
                continue;
            }
        };

        // Validate reserved field names
        for key in doc.fields.keys() {
            if key.starts_with("_") && key != "_id" {
                error_bag.push(ValidationError::ReservedFieldName {
                    doc_id: id.clone(),
                    field: key.clone(),
                });
            }
        }

        // Validate required fields
        for (key, spec) in schema.fields() {
            if spec.required && doc.fields.get(key).is_none() {
                error_bag.push(ValidationError::MissingField {
                    doc_id: id.clone(),
                    field: key.clone(),
                });
            }
        }

        // Validate data types
        for (key, spec) in schema.fields() {
            match (spec.data_type, doc.fields.get(key)) {
                (
                    Some(FieldType {
                        data_type: Some(ref dt),
                    }),
                    Some(data::Value { value: Some(v) }),
                ) => {
                    if !types_match(dt, v) {
                        error_bag.push(ValidationError::InvalidDataType {
                            doc_id: id.clone(),
                            field: key.clone(),
                            expected_type: format!("{:?}", dt),
                            got_value: format!(
                                "{:?}",
                                data::Value {
                                    value: Some(v.clone())
                                }
                            ),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    if !error_bag.is_empty() {
        return Err(error_bag);
    }

    Ok(())
}

pub fn validate_document_ids(ids: &[String]) -> Result<(), ValidationErrorBag<ValidationError>> {
    if ids.is_empty() {
        return Err(ValidationErrorBag::from([ValidationError::NoDocuments]));
    }

    for id in ids {
        if id.is_empty() {
            return Err(ValidationErrorBag::from([ValidationError::InvalidId {
                got: id.clone(),
                doc_offset: 0,
            }]));
        }
    }

    Ok(())
}

fn types_match(spec: &field_type::DataType, value: &data::value::Value) -> bool {
    match (spec, value) {
        (field_type::DataType::Text(..), data::value::Value::String(..)) => true,
        (field_type::DataType::Integer(..), data::value::Value::U32(..)) => true,
        (field_type::DataType::Integer(..), data::value::Value::I32(..)) => true,
        (field_type::DataType::Integer(..), data::value::Value::U64(..)) => true,
        (field_type::DataType::Integer(..), data::value::Value::I64(..)) => true,
        (field_type::DataType::Float(..), data::value::Value::F32(..)) => true,
        (field_type::DataType::Float(..), data::value::Value::F64(..)) => true,
        (field_type::DataType::Boolean(..), data::value::Value::Bool(..)) => true,
        (field_type::DataType::Bytes(..), data::value::Value::Binary(..)) => true,
        (field_type::DataType::FloatVector(dt), data::value::Value::Vector(v)) => match v.len() {
            Some(len) => dt.dimension == len as u32,
            None => false,
        },
        (field_type::DataType::ByteVector(dt), data::value::Value::Vector(v)) => match v.len() {
            Some(len) => dt.dimension == len as u32,
            None => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1::control::{
        field_type::DataType, FieldType, FieldTypeBoolean, FieldTypeBytes, FieldTypeFloat,
        FieldTypeInteger, FieldTypeText,
    };
    use std::collections::HashMap;

    #[test]
    fn test_validate_empty_documents() {
        let errors =
            validate_documents(&CollectionSchema::default(), &vec![]).expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([ValidationError::NoDocuments])
        );
    }

    #[test]
    fn test_validate_documents_missing_id() {
        let errors = validate_documents(
            &CollectionSchema::default(),
            &[data::Document {
                fields: HashMap::new(),
            }],
        )
        .expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([ValidationError::MissingId { doc_offset: 0 }])
        );
    }

    #[test]
    fn test_validate_documents_wrong_id_type() {
        let errors = validate_documents(
            &CollectionSchema::default(),
            &vec![data::Document::from([
                ("_id", data::Value::u32(1)),
                ("data", data::Value::string("x".repeat(1 * 1024))),
            ])],
        )
        .expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([ValidationError::InvalidId {
                got: format!("{:?}", data::Value::u32(1)),
                doc_offset: 0,
            }])
        );
    }

    #[test]
    fn test_validate_documents_reserved_field_name() {
        let errors = validate_documents(
            &CollectionSchema::default(),
            &vec![data::Document::from([
                ("_id", data::Value::string("1".to_string())),
                ("_reserved", data::Value::string("foo".to_string())),
            ])],
        )
        .expect_err("reserved field");

        assert_eq!(
            errors,
            ValidationErrorBag::from([ValidationError::ReservedFieldName {
                doc_id: "1".to_string(),
                field: "_reserved".to_string(),
            }])
        );
    }

    #[test]
    fn test_validate_documents_missing_field() {
        let errors = validate_documents(
            &CollectionSchema::try_from([(
                "age".to_string(),
                FieldSpec {
                    data_type: Some(FieldType {
                        data_type: Some(DataType::Integer(FieldTypeInteger {})),
                    }),
                    required: true,
                    index: None,
                },
            )])
            .unwrap(),
            &vec![data::Document::from([(
                "_id",
                data::Value::string("1".to_string()),
            )])],
        )
        .expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([ValidationError::MissingField {
                doc_id: "1".to_string(),
                field: "age".to_string(),
            }])
        );
    }

    #[rstest::rstest]
    #[case(DataType::Integer(FieldTypeInteger {}), data::Value::string("foo".to_string()))]
    #[case(DataType::Text(FieldTypeText {}), data::Value::u32(3))]
    #[case(DataType::Float(FieldTypeFloat {}), data::Value::u32(3))]
    #[case(DataType::Boolean(FieldTypeBoolean {}), data::Value::u32(3))]
    #[case(DataType::Bytes(FieldTypeBytes {}), data::Value::u32(3))]
    #[case(DataType::FloatVector(FieldTypeFloatVector { dimension: 3 }), data::Value::binary(vec![0,1,2]))]
    #[case(DataType::ByteVector(FieldTypeByteVector { dimension: 3 }), data::Value::binary(vec![0,1,2]))]
    fn test_validate_documents_invalid_data_type(
        #[case] data_type: DataType,
        #[case] value: data::Value,
    ) {
        let errors = validate_documents(
            &CollectionSchema::try_from([(
                "field".to_string(),
                FieldSpec {
                    data_type: Some(FieldType {
                        data_type: Some(data_type),
                    }),
                    required: true,
                    index: None,
                },
            )])
            .unwrap(),
            &vec![data::Document::from([
                ("_id", data::Value::string("1".to_string())),
                ("field", value.clone()),
            ])],
        )
        .expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([ValidationError::InvalidDataType {
                doc_id: "1".to_string(),
                field: "field".to_string(),
                expected_type: format!("{:?}", data_type),
                got_value: format!("{:?}", value),
            }])
        );
    }

    #[test]
    fn test_validate_wrong_vector_dimension() {
        let errors = validate_documents(
            &CollectionSchema::try_from([(
                "field",
                FieldSpec {
                    data_type: Some(FieldType {
                        data_type: Some(DataType::FloatVector(FieldTypeFloatVector {
                            dimension: 3,
                        })),
                    }),
                    required: true,
                    index: None,
                },
            )])
            .unwrap(),
            &vec![data::Document::from([
                ("_id", data::Value::string("1".to_string())),
                ("field", data::Value::float_vector(vec![0.0, 1.0, 2.0, 3.0])),
            ])],
        )
        .expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([ValidationError::InvalidDataType {
                doc_id: "1".to_string(),
                field: "field".to_string(),
                expected_type: format!(
                    "{:?}",
                    DataType::FloatVector(FieldTypeFloatVector { dimension: 3 })
                ),
                got_value: format!("{:?}", data::Value::float_vector(vec![0.0, 1.0, 2.0, 3.0])),
            }])
        );
    }

    #[test]
    fn test_validate_document_ids_no_documents() {
        let errors = validate_document_ids(&vec![]).expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([ValidationError::NoDocuments])
        );
    }

    #[test]
    fn test_validate_document_ids_empty() {
        let errors = validate_document_ids(&vec!["".to_string()]).expect_err("should fail");

        assert_eq!(
            errors,
            ValidationErrorBag::from([ValidationError::InvalidId {
                got: "".to_string(),
                doc_offset: 0,
            }])
        );
    }
}
