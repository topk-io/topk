use std::collections::HashMap;

use prost::Message;
use topk_rs::proto::v1::data::{value::Value as Inner, Document, Value};

use crate::import::coerce::text;
use crate::import::error::{Error, MAX_DOC_BYTES};
use crate::import::source::Record;
use crate::import::spec::Target;
use crate::import::ID;

impl Target {
    /// This row's partition, from the value of the `partition` column, and its document.
    pub fn document(&self, record: Record) -> Result<(Option<String>, Document), Error> {
        let fail = |id: Option<&str>, field: Option<&str>, source: Error| Error::Doc {
            id: id.map(str::to_string),
            field: field.map(str::to_string),
            source: Box::new(source),
        };
        let partition = if let Some(column) = self.partition.as_deref() {
            let value = record
                .iter()
                .find(|(key, _)| key == column)
                .map(|(_, value)| value)
                .filter(|value| value.as_null().is_none())
                .ok_or_else(|| {
                    fail(
                        None,
                        Some(column),
                        Error::InvalidArgument("partition column is missing or null".to_string()),
                    )
                })?;
            let partition = text(value.clone()).map_err(|e| fail(None, Some(column), e))?;
            let mut chars = partition.chars();
            if partition.len() > 128
                || !matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
                || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            {
                return Err(fail(
                    None,
                    Some(column),
                    Error::InvalidArgument(format!(
                        "partition value {partition:?} must start with a letter or digit, \
                     contain only letters, digits, `_` or `-`, and be at most 128 bytes"
                    )),
                ));
            }
            Some(partition)
        } else {
            None
        };
        let id_column = self.id.as_str();
        let id = match record.iter().find(|(key, _)| key == id_column) {
            Some((_, value)) => id_string(id_column, value.clone())?,
            None => {
                let seen: Vec<_> = record.iter().map(|(key, _)| key.as_str()).collect();
                return Err(fail(
                    None,
                    Some(id_column),
                    Error::InvalidArgument(format!(
                        "id column not present in this row, which has: {}",
                        seen.join(", ")
                    )),
                ));
            }
        };
        // The spec is a whitelist; several fields may read one column, the id included.
        let mut doc = Document {
            fields: HashMap::with_capacity(self.fields.len() + 1),
        };
        for (name, field) in &self.fields {
            let missing = || {
                fail(
                    Some(&id),
                    Some(name),
                    Error::InvalidArgument("required field is missing".to_string()),
                )
            };
            let value = match record.iter().find(|(key, _)| key == field.source(name)) {
                Some((_, value)) => value,
                None if field.schema.required => return Err(missing()),
                None => continue,
            };
            let value = field
                .coerce(value.clone())
                .map_err(|e| fail(Some(&id), Some(name), e))?;
            if field.schema.required && value.as_null().is_some() {
                return Err(missing());
            }
            doc.fields.insert(name.clone(), value);
        }

        doc.fields.insert(ID.to_string(), Value::string(id.clone()));
        let size = doc.encoded_len();
        if size > MAX_DOC_BYTES {
            let field = doc
                .fields
                .iter()
                .max_by_key(|(name, value)| name.len() + value.encoded_len())
                .map(|(name, _)| name.clone())
                .unwrap_or_default();
            return Err(Error::Oversized { id, size, field });
        }
        Ok((partition, doc))
    }
}

/// A document id from a source value: text as-is, numbers exactly.
pub fn id_string(id: &str, value: Value) -> Result<String, Error> {
    let fail = |source: Error| Error::Doc {
        id: None,
        field: Some(id.to_string()),
        source: Box::new(source),
    };
    let invalid = |message: String| fail(Error::InvalidArgument(message));
    if value.as_null().is_some() {
        return Err(invalid("id is null".to_string()));
    }
    if let Some(f) = match value.value.as_ref() {
        Some(Inner::F32(f)) => Some(*f as f64),
        Some(Inner::F64(f)) => Some(*f),
        _ => None,
    } {
        if !f.is_finite() {
            return Err(invalid(
                "non-finite numeric value cannot be a document id".to_string(),
            ));
        }
        // Beyond 2^53 a double has dropped digits; the id would be wrong.
        if f.abs() >= (1u64 << 53) as f64 {
            return Err(invalid(format!(
                "{f} came through as a double and lost integer precision; \
                 cast the column to text or integer in the source"
            )));
        }
    }
    let rendered = text(value).map_err(fail)?;
    if rendered.is_empty() {
        return Err(invalid("empty value cannot be a document id".to_string()));
    }
    Ok(rendered)
}
