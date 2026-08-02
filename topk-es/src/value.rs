use std::cmp::Ordering;

use topk_rs::proto::v1::data::{list, value, List, Value};

pub struct OrdValue(pub Value);

pub fn compare(a: &Value, b: &Value) -> Ordering {
    if let (Some(x), Some(y)) = (a.number(), b.number()) {
        return x.total_cmp(&y);
    }
    if let (Some(x), Some(y)) = (a.as_string(), b.as_string()) {
        return x.cmp(y);
    }
    if let (Some(x), Some(y)) = (a.as_bool(), b.as_bool()) {
        return x.cmp(&y);
    }
    Ordering::Equal
}

impl Ord for OrdValue {
    fn cmp(&self, other: &Self) -> Ordering {
        compare(&self.0, &other.0)
    }
}

impl PartialOrd for OrdValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for OrdValue {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for OrdValue {}

// Coercions a value undergoes on its way to (or from) a typed collection field.
// Each one consumes the value and hands it back untouched — `Err(value)` — when
// it does not apply, so a caller that can live without the coercion pays no
// clone, and one that cannot has its rejection.
pub trait ValueExt: Sized {
    fn number(&self) -> Option<f64>;
    fn is_scalar(&self) -> bool;
    fn into_f32_list(self) -> Result<Self, Self>;
    fn into_i8_list(self) -> Result<Self, Self>;
    fn into_unsigned_bytes(self) -> Result<Self, Self>;
    fn into_u8_matrix(self) -> Result<Self, Self>;
    fn into_signed_bytes(self) -> Self;
}

impl ValueExt for Value {
    fn is_scalar(&self) -> bool {
        matches!(
            self.value,
            Some(
                value::Value::Bool(_)
                    | value::Value::String(_)
                    | value::Value::U32(_)
                    | value::Value::U64(_)
                    | value::Value::I32(_)
                    | value::Value::I64(_)
                    | value::Value::F32(_)
                    | value::Value::F64(_)
            )
        )
    }

    fn number(&self) -> Option<f64> {
        match &self.value {
            Some(value::Value::F64(v)) => Some(*v),
            Some(value::Value::F32(v)) => Some(f64::from(*v)),
            Some(value::Value::I64(v)) => Some(*v as f64),
            Some(value::Value::I32(v)) => Some(f64::from(*v)),
            Some(value::Value::U64(v)) => Some(*v as f64),
            Some(value::Value::U32(v)) => Some(f64::from(*v)),
            _ => None,
        }
    }

    fn into_f32_list(self) -> Result<Self, Self> {
        if self.as_f32_list().is_some() {
            return Ok(self);
        }

        match int_list(&self, |n| n as f32) {
            Some(values) => Ok(Value::list(values)),
            None => Err(self),
        }
    }

    fn into_i8_list(self) -> Result<Self, Self> {
        match byte_list(&self, |n| n as i8) {
            Some(values) => Ok(Value::list(values)),
            None => Err(self),
        }
    }

    // Signed bytes wrapped into their unsigned storage form; inverse of
    // `into_signed_bytes`.
    fn into_unsigned_bytes(self) -> Result<Self, Self> {
        match byte_list(&self, |n| n as u8) {
            Some(values) => Ok(Value::list(values)),
            None => Err(self),
        }
    }

    fn into_u8_matrix(self) -> Result<Self, Self> {
        match u8_matrix(&self) {
            Some((num_cols, values)) => Ok(Value::matrix(num_cols, values)),
            None => Err(self),
        }
    }

    // Reinterpret a u8 list as the signed bytes it was encoded from.
    fn into_signed_bytes(self) -> Self {
        match self.value {
            Some(value::Value::List(List {
                values: Some(list::Values::U8(values)),
            })) => Value::list(
                values
                    .values
                    .into_iter()
                    .map(|v| v as i8)
                    .collect::<Vec<_>>(),
            ),
            _ => self,
        }
    }
}

// The helpers below hand back owned values so the borrow of `value` ends with
// the call, leaving the caller free to move it into the `None` arm.
fn int_list<T>(value: &Value, cast: impl Fn(i64) -> T) -> Option<Vec<T>> {
    Some(value.as_i64_list()?.iter().map(|&n| cast(n)).collect())
}

fn byte_list<T>(value: &Value, cast: impl Fn(i64) -> T) -> Option<Vec<T>> {
    let ints = value.as_i64_list()?;
    ints.iter()
        .all(|n| (-128..=127).contains(n))
        .then(|| ints.iter().map(|&n| cast(n)).collect())
}

fn u8_matrix(value: &Value) -> Option<(u32, Vec<u8>)> {
    let (_, num_cols, values) = value.as_f32_matrix()?;

    values
        .iter()
        .all(|v| v.is_finite() && v.fract() == 0.0 && (0.0..=255.0).contains(v))
        .then(|| (num_cols, values.iter().map(|&v| v as u8).collect()))
}
