use super::*;

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
}

impl<T: IntoIterator<Item = (K, Value)>, K: Into<String>> From<T> for Document {
    fn from(entries: T) -> Self {
        Document {
            fields: entries.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }
}
