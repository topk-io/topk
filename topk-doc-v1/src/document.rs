use std::collections::HashMap;

use rkyv::{Archive, Deserialize, Serialize, rancor::Error as RkyvError};

use super::{DocumentError, Value};

#[derive(Archive, Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub fields: HashMap<String, Value>,
}

impl Document {
    /// Returns document _id field.
    #[inline]
    pub fn id(&self) -> Result<&str, DocumentError> {
        match self.fields.get("_id") {
            Some(val) => match val {
                Value::String(id) => Ok(id),
                v => Err(DocumentError::InvalidId(v.clone())),
            },
            _ => Err(DocumentError::MissingId),
        }
    }

    #[inline(always)]
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(rkyv::to_bytes::<RkyvError>(self)?.to_vec())
    }

    #[inline(always)]
    pub fn decode(data: &[u8]) -> anyhow::Result<Document> {
        Ok(rkyv::from_bytes::<_, RkyvError>(data)?)
    }

    #[inline(always)]
    pub fn access<'a>(data: &'a [u8]) -> anyhow::Result<&'a ArchivedDocument> {
        Ok(rkyv::access::<ArchivedDocument, RkyvError>(data)?)
    }
}

impl<K: Into<String>, T: IntoIterator<Item = (K, Value)>> From<T> for Document {
    fn from(entries: T) -> Self {
        Document {
            fields: entries.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }
}
