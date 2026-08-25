pub mod arrow;
pub mod bson;
pub mod es;

use topk_rs::proto::v1::data::Value;

use crate::import::error::Error;

pub fn finite(f: f64) -> Result<f64, Error> {
    if f.is_finite() {
        Ok(f)
    } else {
        Err(Error::InvalidArgument(format!("non-finite float {f}")))
    }
}

/// Exact i64 from a float, by round-trip: rejects fractions, NaN/inf, and
/// out-of-range. The one lie it accepts: exactly 2^63 saturates to i64::MAX.
pub fn int_from_f64(f: f64) -> Option<i64> {
    (f as i64 as f64 == f).then_some(f as i64)
}

/// Exact i64 from a string, accepting an all-zero fraction ("3", " 3.00 ").
pub fn int_from_str(s: &str) -> Option<i64> {
    let s = s.trim();
    let (int, frac) = s.split_once('.').unwrap_or((s, "0"));
    if frac.is_empty() || frac.bytes().any(|b| b != b'0') {
        return None;
    }
    int.parse().ok()
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
                .and_then(|v| v.iter().map(|s| int_from_str(s)).collect())
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

/// A document id from a source value: text as-is, numbers exactly.
pub fn id_string(id: &str, value: Value) -> Result<String, Error> {
    if value.as_null().is_some() {
        return Err(Error::Id(id.to_string(), "id is null".to_string()));
    }
    let rendered = match serde_json::Value::try_from(value) {
        Ok(serde_json::Value::String(s)) => s,
        // Excel stores every number as a double; "1.0" and "1" must be one id.
        Ok(serde_json::Value::Number(n)) if n.is_f64() => {
            let f = n.as_f64().unwrap();
            match int_from_f64(f) {
                Some(i) => i.to_string(),
                // Beyond 2^53 a double has dropped digits; the id would be wrong.
                None if f.abs() >= (1u64 << 53) as f64 => {
                    return Err(Error::Id(
                        id.to_string(),
                        format!(
                            "{f} came through as a double and lost integer precision; \
                             cast the column to text or integer in the source"
                        ),
                    ))
                }
                None => f.to_string(),
            }
        }
        Ok(other) => other.to_string(),
        Err(_) => {
            return Err(Error::Id(
                id.to_string(),
                "non-finite numeric value cannot be a document id".to_string(),
            ))
        }
    };
    if rendered.is_empty() {
        return Err(Error::Id(
            id.to_string(),
            "empty value cannot be a document id".to_string(),
        ));
    }
    Ok(rendered)
}
