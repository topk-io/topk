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
