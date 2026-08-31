mod coerce;
mod decode;

use topk_rs::proto::v1::data::Value;

use crate::import::error::Error;

pub use decode::floats;
use decode::{float, int, text};

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
