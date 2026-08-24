use chrono::{DateTime, SecondsFormat, Utc};
use topk_rs::proto::v1::control::{field_type, FieldSpec};
use topk_rs::proto::v1::data::Value;

use crate::Error;

// ES `date` values arrive as ISO-8601 strings (or already-epoch millis) and must land in TopK's
// timestamp column as i64 millis; reads go the other way. Date math (`now-30s`) needs the wall
// clock and is not supported yet — it is rejected rather than silently mis-parsed.
fn parse_millis(value: &str) -> Result<i64, Error> {
    if let Ok(millis) = value.parse::<i64>() {
        return Ok(millis);
    }

    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.timestamp_millis())
        .map_err(|_| Error::BadRequest(format!("cannot parse date [{value}]")))
}

pub fn format_millis(millis: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(millis)
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
}

pub fn is_timestamp(spec: &FieldSpec) -> bool {
    matches!(
        spec.data_type.as_ref().and_then(|t| t.data_type.as_ref()),
        Some(field_type::DataType::Timestamp(_))
    )
}

// Coerce a value destined for `spec` onto the epoch millis a timestamp column stores. ES accepts
// an ISO-8601 string, raw millis, or a list of either (`terms`). Non-timestamp fields pass through
// untouched, so a numeric-looking string on a keyword field is never mistaken for a date.
pub fn to_timestamp(spec: Option<&FieldSpec>, value: Value) -> Result<Value, Error> {
    if !spec.is_some_and(is_timestamp) {
        return Ok(value);
    }

    if let Some(s) = value.as_string() {
        return Ok(Value::timestamp(parse_millis(s)?));
    }

    match value.as_string_list() {
        Some(values) => values
            .iter()
            .map(|v| parse_millis(v))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::list),
        None => Ok(value),
    }
}

// Inverse of `to_timestamp`: render a stored timestamp back as the ISO-8601 string ES returns.
// A value that isn't a usable timestamp reads as null rather than leaking raw millis.
pub fn from_timestamp(value: Value) -> Value {
    match value.as_timestamp().and_then(format_millis) {
        Some(iso) => Value::string(iso),
        None => Value::null(),
    }
}
