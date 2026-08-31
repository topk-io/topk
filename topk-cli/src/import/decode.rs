use topk_rs::proto::v1::data::{value::Value as Inner, Value};

use crate::import::error::Error;

pub fn text(value: Value) -> Result<String, Error> {
    Ok(match value.value {
        Some(Inner::String(s)) => s,
        Some(Inner::Binary(bytes)) => String::from_utf8(bytes.into()).map_err(|_| {
            Error::InvalidArgument(
                "declared as text but the bytes are not valid UTF-8; declare `bytes` to \
                 keep them as binary"
                    .to_string(),
            )
        })?,
        Some(Inner::Bool(b)) => b.to_string(),
        Some(Inner::I32(n)) => n.to_string(),
        Some(Inner::I64(n)) => n.to_string(),
        Some(Inner::U32(n)) => n.to_string(),
        Some(Inner::U64(n)) => n.to_string(),
        Some(Inner::F32(f)) => f.to_string(),
        Some(Inner::F64(f)) => f.to_string(),
        value => serde_json::Value::try_from(Value { value })?.to_string(),
    })
}

/// An exact i64 from whatever numeric shape a source produced.
pub fn int(value: &Value) -> Option<i64> {
    match value.value.as_ref()? {
        Inner::I32(n) => Some(*n as i64),
        Inner::I64(n) => Some(*n),
        Inner::U32(n) => Some(*n as i64),
        Inner::U64(n) => i64::try_from(*n).ok(),
        Inner::F32(f) => exact_int(*f as f64),
        Inner::F64(f) => exact_int(*f),
        Inner::Bool(b) => Some(*b as i64),
        Inner::String(s) => parse_int(s),
        _ => None,
    }
}

pub fn float(value: &Value) -> Option<f64> {
    match value.value.as_ref()? {
        Inner::I32(n) => Some(*n as f64),
        Inner::I64(n) => Some(*n as f64),
        Inner::U32(n) => Some(*n as f64),
        Inner::U64(n) => Some(*n as f64),
        Inner::F32(f) => Some(*f as f64),
        Inner::F64(f) => Some(*f),
        Inner::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Exact i64 from a float, by round-trip: rejects fractions, NaN/inf, and
/// out-of-range. The one lie it accepts: exactly 2^63 saturates to i64::MAX.
pub fn exact_int(f: f64) -> Option<i64> {
    (f as i64 as f64 == f).then_some(f as i64)
}

/// Exact i64 from a rendered number; an all-zero fraction is dropped (" 3.00 ").
/// Parsed as an integer, not through f64, so ids past 2^53 stay exact.
fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim();
    match s.split_once('.') {
        None => s.parse().ok(),
        Some((int, frac)) if !frac.is_empty() && frac.bytes().all(|b| b == b'0') => {
            int.parse().ok()
        }
        Some(_) => None,
    }
}

pub fn ints(value: &Value) -> Option<Vec<i64>> {
    value
        .as_i64_list()
        .map(|v| v.to_vec())
        .or_else(|| {
            value
                .as_i32_list()
                .map(|v| v.iter().map(|&n| n as i64).collect())
        })
        .or_else(|| {
            value
                .as_u32_list()
                .map(|v| v.iter().map(|&n| n as i64).collect())
        })
        .or_else(|| {
            value
                .as_u64_list()
                .and_then(|v| v.iter().map(|&n| i64::try_from(n).ok()).collect())
        })
        .or_else(|| {
            value
                .as_u8_list()
                .map(|v| v.iter().map(|&n| n as i64).collect())
        })
        .or_else(|| {
            value
                .as_i8_list()
                .map(|v| v.iter().map(|&n| n as i64).collect())
        })
        .or_else(|| {
            value
                .as_string_list()
                .and_then(|v| v.iter().map(|s| parse_int(s)).collect())
        })
}

pub fn floats(value: &Value) -> Option<Vec<f64>> {
    value
        .as_f64_list()
        .map(|v| v.to_vec())
        .or_else(|| {
            value
                .as_f32_list()
                .map(|v| v.iter().map(|&n| n as f64).collect())
        })
        .or_else(|| {
            value
                .as_u64_list()
                .map(|v| v.iter().map(|&n| n as f64).collect())
        })
        .or_else(|| ints(value).map(|v| v.iter().map(|&n| n as f64).collect()))
        .or_else(|| {
            value
                .as_string_list()
                .and_then(|v| v.iter().map(|s| s.trim().parse().ok()).collect())
        })
}

pub fn finite(f: f64) -> Result<f64, Error> {
    match f.is_finite() {
        true => Ok(f),
        false => Err(Error::InvalidArgument(format!("non-finite float {f}"))),
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
