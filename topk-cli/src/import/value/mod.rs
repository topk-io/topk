mod coerce;
mod decode;

use prost::Message;
use topk_rs::proto::v1::data::{Document, Value};

use crate::import::error::{Error, MAX_DOC_BYTES};
use crate::import::source::Record;
use crate::import::spec::Target;
use crate::import::ID;

pub use decode::floats;
use decode::{float, int, text};

/// The document a target asks for, built from one source row.
pub fn document(target: &Target, record: Record) -> Result<Document, Error> {
    let id_column = target.id.as_deref().unwrap_or(ID);
    let id = match record.iter().find(|(key, _)| key == id_column) {
        Some((_, value)) => id_string(id_column, value.clone())?,
        None => {
            let seen: Vec<_> = record.iter().map(|(key, _)| key.as_str()).collect();
            return Err(Error::Doc {
                id: None,
                field: Some(id_column.to_string()),
                source: Box::new(Error::InvalidArgument(format!(
                    "id column not present in this row, which has: {}",
                    seen.join(", ")
                ))),
            });
        }
    };
    let fail = |field: Option<&str>, source: Error| Error::Doc {
        id: Some(id.clone()),
        field: field.map(str::to_string),
        source: Box::new(source),
    };

    // The spec is a whitelist; several fields may read one column, the id included.
    let mut pairs: Vec<(String, Value)> = Vec::with_capacity(target.fields.len() + 1);
    for (key, value) in record {
        for (name, field) in target
            .fields
            .iter()
            .filter(|(name, field)| field.from.as_deref().unwrap_or(name.as_str()) == key)
        {
            let value = field
                .coerce(value.clone())
                .map_err(|e| fail(Some(name), e))?;
            pairs.push((name.clone(), value));
        }
    }
    for (name, field) in &target.fields {
        if field.required
            && !pairs
                .iter()
                .any(|(key, value)| key == name && value.as_null().is_none())
        {
            return Err(fail(
                Some(name),
                Error::InvalidArgument("required field is missing".to_string()),
            ));
        }
    }

    pairs.push((ID.to_string(), Value::string(id.clone())));
    let doc = Document::from(pairs);
    let size = doc.encoded_len();
    if size > MAX_DOC_BYTES {
        return Err(fail(None, Error::Oversized(size)));
    }
    Ok(doc)
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
    if let Some(f) = float(&value).filter(|_| int(&value).is_none()) {
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
