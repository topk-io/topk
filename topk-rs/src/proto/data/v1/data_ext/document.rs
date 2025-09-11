use std::collections::HashMap;

use crate::proto::data::v1::{document, value, Document, Value};

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("Missing document _id field")]
    MissingId,

    #[error("Invalid document _id field: {0:?}")]
    InvalidId(value::Value),
}

impl Document {
    /// Returns document _id field.
    #[inline]
    pub fn id(&self) -> Result<&str, DocumentError> {
        match self.fields.get("_id").map(|v| &v.value) {
            Some(Some(val)) => match val {
                value::Value::String(id) => Ok(id),
                v => Err(DocumentError::InvalidId(v.clone())),
            },
            _ => Err(DocumentError::MissingId),
        }
    }

    /// Returns document fields.
    #[inline]
    pub(crate) fn into_fields(self) -> HashMap<String, crate::doc::Value> {
        match self.data {
            Some(document::Data::V1(data)) => {
                assert!(
                    self.fields.is_empty(),
                    "Document fields must be empty when data is present"
                );
                let doc = crate::doc::Document::decode(&data).expect("Failed to decode document");
                doc.fields
            }
            None => self
                .fields
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }

    pub(crate) fn encode(doc: crate::doc::Document) -> Document {
        let data = doc.encode().expect("Failed to encode document");

        Document {
            fields: Default::default(),
            data: Some(document::Data::V1(data)),
        }
    }
}

impl From<Document> for crate::doc::Document {
    fn from(doc: Document) -> Self {
        crate::doc::Document {
            fields: doc.into_fields(),
        }
    }
}

impl<T: IntoIterator<Item = (K, Value)>, K: Into<String>> From<T> for Document {
    fn from(entries: T) -> Self {
        Document {
            fields: entries.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            data: None,
        }
    }
}
