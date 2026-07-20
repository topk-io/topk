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

pub trait ValueExt: Sized {
    fn number(&self) -> Option<f64>;
    fn is_scalar(&self) -> bool;
    fn to_f32_list(&self) -> Option<Self>;
    fn to_i8_list(&self) -> Option<Self>;
    fn to_unsigned_bytes(&self) -> Option<Self>;
    fn to_u8_matrix(&self) -> Option<Self>;
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
        self.as_f64()
            .or_else(|| self.as_f32().map(f64::from))
            .or_else(|| self.as_i64().map(|v| v as f64))
            .or_else(|| self.as_i32().map(f64::from))
            .or_else(|| self.as_u64().map(|v| v as f64))
            .or_else(|| self.as_u32().map(f64::from))
    }

    fn to_f32_list(&self) -> Option<Self> {
        if self.as_f32_list().is_some() {
            return Some(self.clone());
        }
        let ints = self.as_i64_list()?;
        Some(Value::list(
            ints.iter().map(|&n| n as f32).collect::<Vec<_>>(),
        ))
    }

    fn to_i8_list(&self) -> Option<Self> {
        let ints = self.as_i64_list()?;
        ints.iter()
            .all(|n| (-128..=127).contains(n))
            .then(|| Value::list(ints.iter().map(|&n| n as i8).collect::<Vec<i8>>()))
    }

    // Signed bytes wrapped into their unsigned storage form; inverse of
    // `into_signed_bytes`.
    fn to_unsigned_bytes(&self) -> Option<Self> {
        let ints = self.as_i64_list()?;
        ints.iter()
            .all(|n| (-128..=127).contains(n))
            .then(|| Value::list(ints.iter().map(|&n| n as u8).collect::<Vec<u8>>()))
    }

    fn to_u8_matrix(&self) -> Option<Self> {
        let (_, num_cols, values) = self.as_f32_matrix()?;
        if !values
            .iter()
            .all(|v| v.is_finite() && v.fract() == 0.0 && (0.0..=255.0).contains(v))
        {
            return None;
        }

        Some(Value::matrix(
            num_cols,
            values.iter().map(|&v| v as u8).collect::<Vec<u8>>(),
        ))
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
