use std::collections::HashMap;
use std::ops::Deref;

use serde::de::Error as DeError;
use serde::ser::Error as SerError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::proto::v1::data::{list, matrix, sparse_vector, value, vector, Value as TopkValue};

macro_rules! array_of {
    ($values:expr, $mapper:expr) => {
        $values
            .into_iter()
            .map($mapper)
            .collect::<Result<Vec<_>, crate::Error>>()
    };
    ($values:expr, $variant:path, $error:expr) => {
        $values
            .into_iter()
            .map(|value| match value {
                $variant(inner) => Ok(inner),
                _ => Err(crate::Error::InvalidArgument($error.into())),
            })
            .collect::<Result<Vec<_>, crate::Error>>()
    };
}

/// JSON wire-format wrapper around a protobuf [`TopkValue`].
#[derive(Clone, Debug, PartialEq)]
pub struct Value(pub TopkValue);

impl<T: Into<TopkValue>> From<T> for Value {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Value {
    pub fn into_inner(self) -> TopkValue {
        self.0
    }
}

impl Deref for Value {
    type Target = TopkValue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::Value::try_from(self.0.clone())
            .map_err(SerError::custom)?
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(
            TopkValue::try_from(serde_json::Value::deserialize(deserializer)?)
                .map_err(DeError::custom)?,
        ))
    }
}

impl TryFrom<serde_json::Value> for TopkValue {
    type Error = crate::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Null => Ok(TopkValue::null()),
            serde_json::Value::Bool(value) => Ok(TopkValue::bool(value)),
            serde_json::Value::String(value) => Ok(TopkValue::string(value)),
            serde_json::Value::Number(value) => Ok(TopkValue::try_from(value)?),
            serde_json::Value::Object(value) => Ok(TopkValue::try_from(value)?),
            serde_json::Value::Array(value) => Ok(TopkValue::try_from(value)?),
        }
    }
}

impl TryFrom<serde_json::Number> for TopkValue {
    type Error = crate::Error;

    fn try_from(value: serde_json::Number) -> Result<Self, Self::Error> {
        if let Some(value) = value.as_i64() {
            Ok(TopkValue::i64(value))
        } else if let Some(value) = value.as_u64() {
            Ok(TopkValue::u64(value))
        } else {
            let value = value.as_f64().ok_or_else(|| {
                crate::Error::InvalidArgument(
                    "JSON number is outside TopK's supported numeric range".into(),
                )
            })?;

            Ok(TopkValue::f64(value))
        }
    }
}

impl TryFrom<Vec<serde_json::Value>> for TopkValue {
    type Error = crate::Error;

    fn try_from(values: Vec<serde_json::Value>) -> Result<Self, Self::Error> {
        let Some(first) = values.first() else {
            return Ok(TopkValue::list(Vec::<i64>::new()));
        };

        if first.is_number() {
            return json_number_array(values);
        }

        if first.is_string() {
            return Ok(TopkValue::list(array_of!(
                values,
                serde_json::Value::String,
                "JSON arrays must contain only numbers or strings"
            )?));
        }

        if first.is_array() {
            return TopkValue::try_from(array_of!(
                values,
                serde_json::Value::Array,
                "JSON arrays must contain only numbers, strings, or numeric arrays"
            )?);
        }

        Err(crate::Error::InvalidArgument(
            "JSON arrays must contain only numbers, strings, or numeric arrays".into(),
        ))
    }
}

impl TryFrom<Vec<Vec<serde_json::Value>>> for TopkValue {
    type Error = crate::Error;

    fn try_from(rows: Vec<Vec<serde_json::Value>>) -> Result<Self, Self::Error> {
        let num_cols = rows
            .first()
            .map(Vec::len)
            .filter(|num_cols| *num_cols > 0)
            .ok_or_else(|| {
                crate::Error::InvalidArgument("JSON matrices must have at least one column".into())
            })?;

        for row in &rows {
            if row.len() != num_cols {
                return Err(crate::Error::InvalidArgument(
                    "JSON matrix rows must have the same length".into(),
                ));
            }
        }

        if rows.len() * num_cols >= u32::MAX as usize {
            return Err(crate::Error::InvalidArgument(
                "JSON matrix has too many values".into(),
            ));
        }

        rows.into_iter()
            .flatten()
            .map(number_to_f32)
            .collect::<Result<Vec<_>, _>>()
            .map(|values| TopkValue::matrix(num_cols as u32, values))
    }
}

impl TryFrom<serde_json::Map<String, serde_json::Value>> for TopkValue {
    type Error = crate::Error;

    fn try_from(object: serde_json::Map<String, serde_json::Value>) -> Result<Self, Self::Error> {
        let is_sparse_vector = !object.is_empty()
            && object
                .iter()
                .all(|(key, value)| key.parse::<u32>().is_ok() && value.is_number());

        if is_sparse_vector {
            let mut indices = Vec::with_capacity(object.len());
            let mut values = Vec::with_capacity(object.len());
            for (key, value) in object {
                indices.push(key.parse::<u32>().expect("keys are valid u32 indices"));
                values.push(number_to_f32(value)?);
            }
            return Ok(TopkValue::f32_sparse_vector(indices, values));
        }

        object
            .into_iter()
            .map(|(key, value)| Ok((key, TopkValue::try_from(value)?)))
            .collect::<Result<HashMap<_, _>, _>>()
            .map(TopkValue::r#struct)
    }
}

impl TryFrom<TopkValue> for serde_json::Value {
    type Error = crate::Error;

    fn try_from(value: TopkValue) -> Result<Self, Self::Error> {
        match value.value {
            None | Some(value::Value::Null(_)) => Ok(Self::Null),
            Some(value::Value::Bool(v)) => Ok(Self::Bool(v)),
            Some(value::Value::String(v)) => Ok(Self::String(v)),
            Some(value::Value::U32(v)) => Ok(Self::from(v)),
            Some(value::Value::U64(v)) => Ok(Self::from(v)),
            Some(value::Value::I32(v)) => Ok(Self::from(v)),
            Some(value::Value::I64(v)) => Ok(Self::from(v)),
            Some(value::Value::F32(v)) => number(v),
            Some(value::Value::F64(v)) => number(v),
            Some(value::Value::Binary(v)) => {
                Ok(Self::Array(v.into_iter().map(Self::from).collect()))
            }
            #[allow(deprecated)]
            Some(value::Value::Vector(v)) => match v.vector {
                Some(vector::Vector::Float(v)) => array_of!(v.values, number).map(Self::Array),
                Some(vector::Vector::Byte(v)) => {
                    Ok(Self::Array(v.values.into_iter().map(Self::from).collect()))
                }
                None => Ok(Self::Null),
            },
            Some(value::Value::Struct(v)) => v
                .fields
                .into_iter()
                .map(|(key, value)| Ok((key, serde_json::Value::try_from(value)?)))
                .collect::<Result<serde_json::Map<_, _>, _>>()
                .map(Self::Object),
            Some(value::Value::List(v)) => {
                let values = match v.values {
                    Some(list::Values::U32(v)) => v.values.into_iter().map(Self::from).collect(),
                    Some(list::Values::U64(v)) => v.values.into_iter().map(Self::from).collect(),
                    Some(list::Values::I32(v)) => v.values.into_iter().map(Self::from).collect(),
                    Some(list::Values::I64(v)) => v.values.into_iter().map(Self::from).collect(),
                    Some(list::Values::F8(v)) => {
                        array_of!(v.as_ref().iter().map(|n| f32::from(*n)), number)?
                    }
                    Some(list::Values::F16(v)) => {
                        array_of!(v.as_ref().iter().map(|n| f32::from(*n)), number)?
                    }
                    Some(list::Values::F32(v)) => array_of!(v.values, number)?,
                    Some(list::Values::F64(v)) => array_of!(v.values, number)?,
                    Some(list::Values::String(v)) => v.values.into_iter().map(Self::from).collect(),
                    Some(list::Values::U8(v)) => v.values.into_iter().map(Self::from).collect(),
                    Some(list::Values::I8(v)) => {
                        v.as_ref().iter().copied().map(Self::from).collect()
                    }
                    None => Vec::new(),
                };

                Ok(Self::Array(values))
            }
            Some(value::Value::SparseVector(v)) => {
                let mut object = serde_json::Map::new();
                match v.values {
                    Some(sparse_vector::Values::F32(values)) => {
                        for (index, value) in v.indices.into_iter().zip(values.values) {
                            object.insert(index.to_string(), number(value)?);
                        }
                    }
                    Some(sparse_vector::Values::F16(values)) => {
                        for (index, value) in v.indices.into_iter().zip(values.as_ref()) {
                            object.insert(index.to_string(), number(f32::from(*value))?);
                        }
                    }
                    Some(sparse_vector::Values::F8(values)) => {
                        for (index, value) in v.indices.into_iter().zip(values.as_ref()) {
                            object.insert(index.to_string(), number(f32::from(*value))?);
                        }
                    }
                    Some(sparse_vector::Values::U8(values)) => {
                        for (index, value) in v.indices.into_iter().zip(values.values) {
                            object.insert(index.to_string(), Self::from(value));
                        }
                    }
                    Some(sparse_vector::Values::I8(values)) => {
                        for (index, value) in v.indices.into_iter().zip(values.as_ref()) {
                            object.insert(index.to_string(), Self::from(*value));
                        }
                    }
                    None => {}
                }
                Ok(Self::Object(object))
            }
            Some(value::Value::Matrix(v)) => {
                let num_cols = v.num_cols as usize;
                if num_cols == 0 {
                    return Ok(Self::Array(Vec::new()));
                }

                let rows = match v.values {
                    Some(matrix::Values::F32(values)) => array_of!(
                        values.values.chunks(num_cols),
                        |row| array_of!(row.iter().copied(), number).map(Self::Array)
                    )?,
                    Some(matrix::Values::F16(values)) => array_of!(
                        values.as_ref().chunks(num_cols),
                        |row| array_of!(row.iter().map(|n| f32::from(*n)), number).map(Self::Array)
                    )?,
                    Some(matrix::Values::F8(values)) => array_of!(
                        values.as_ref().chunks(num_cols),
                        |row| array_of!(row.iter().map(|n| f32::from(*n)), number).map(Self::Array)
                    )?,
                    Some(matrix::Values::U8(values)) => values
                        .values
                        .chunks(num_cols)
                        .map(|row| Self::Array(row.iter().copied().map(Self::from).collect()))
                        .collect(),
                    Some(matrix::Values::I8(values)) => values
                        .as_ref()
                        .chunks(num_cols)
                        .map(|row| Self::Array(row.iter().copied().map(Self::from).collect()))
                        .collect(),
                    None => Vec::new(),
                };

                Ok(Self::Array(rows))
            }
        }
    }
}

fn json_number_array(values: Vec<serde_json::Value>) -> Result<TopkValue, crate::Error> {
    let mut ints = Vec::with_capacity(values.len());
    for value in &values {
        match json_whole_number_as_i64(value)? {
            Some(n) => ints.push(n),
            None => {
                return values
                    .into_iter()
                    .map(number_to_f32)
                    .collect::<Result<Vec<_>, _>>()
                    .map(TopkValue::list);
            }
        }
    }

    Ok(TopkValue::list(ints))
}

fn json_whole_number_as_i64(value: &serde_json::Value) -> Result<Option<i64>, crate::Error> {
    let serde_json::Value::Number(number) = value else {
        return Err(crate::Error::InvalidArgument(
            "JSON arrays must contain only numbers or strings".into(),
        ));
    };

    if let Some(value) = number.as_i64() {
        return Ok(Some(value));
    }

    if let Some(value) = number.as_u64() {
        return Ok((value <= i64::MAX as u64).then_some(value as i64));
    }

    let value = number.as_f64().ok_or_else(|| {
        crate::Error::InvalidArgument(
            "JSON number is outside TopK's supported numeric range".into(),
        )
    })?;

    if !(value.is_finite() && value.fract() == 0.0) {
        return Ok(None);
    }

    if value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        Ok(Some(value as i64))
    } else {
        Ok(None)
    }
}

fn number_to_f32(value: serde_json::Value) -> Result<f32, crate::Error> {
    // Values beyond f32's range become infinity when downcast, matching the JS/Python SDKs.
    Ok(value.as_f64().ok_or_else(|| {
        crate::Error::InvalidArgument("JSON arrays must contain only numbers or strings".into())
    })? as f32)
}

fn number(value: impl Into<f64>) -> Result<serde_json::Value, crate::Error> {
    serde_json::Number::from_f64(value.into())
        .map(serde_json::Value::Number)
        .ok_or_else(|| crate::Error::InvalidArgument("non-finite floating-point value".into()))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    fn f16s(values: &[f32]) -> Vec<half::f16> {
        values.iter().copied().map(half::f16::from_f32).collect()
    }

    fn f8s(values: &[f32]) -> Vec<float8::F8E4M3> {
        values
            .iter()
            .copied()
            .map(float8::F8E4M3::from_f32)
            .collect()
    }

    #[rstest]
    // scalars
    #[case::i64(json!(42), TopkValue::i64(42))]
    #[case::i64_negative(json!(-5), TopkValue::i64(-5))]
    #[case::u64(
        serde_json::Value::Number(serde_json::Number::from(i64::MAX as u64 + 1)),
        TopkValue::u64(i64::MAX as u64 + 1)
    )]
    #[case::f64(json!(0.1), TopkValue::f64(0.1))]
    // arrays
    #[case::int_list(json!([1, 2]), TopkValue::list(vec![1_i64, 2]))]
    #[case::negative_int_list(json!([-1, 2]), TopkValue::list(vec![-1_i64, 2]))]
    #[case::byte_value_list(json!([255, 128, 1, 0]), TopkValue::list(vec![255_i64, 128, 1, 0]))]
    #[case::whole_float_list(json!([1.0, 2.0]), TopkValue::list(vec![1_i64, 2]))]
    #[case::mixed_list(json!([1, 2.5]), TopkValue::list(vec![1.0_f32, 2.5]))]
    #[case::u64_overflow_list(
        json!([1, i64::MAX as u64 + 1]),
        TopkValue::list(vec![1.0_f32, (i64::MAX as u64 + 1) as f32])
    )]
    #[case::empty_list(json!([]), TopkValue::list(Vec::<i64>::new()))]
    #[case::string_list(json!(["a", "b"]), TopkValue::list(vec!["a", "b"]))]
    // matrices
    #[case::matrix(
        json!([[1, 2], [3.5, 4]]),
        TopkValue::matrix(2, vec![1.0_f32, 2.0, 3.5, 4.0])
    )]
    // objects
    #[case::sparse_vector(
        json!({"0": 1.5, "2": 3.0}),
        TopkValue::f32_sparse_vector(vec![0, 2], vec![1.5, 3.0])
    )]
    #[case::struct_value(
        json!({"name": "a", "count": 2}),
        TopkValue::r#struct([("name", TopkValue::string("a")), ("count", TopkValue::i64(2))])
    )]
    #[case::empty_object(
        json!({}),
        TopkValue::r#struct(Vec::<(String, TopkValue)>::new())
    )]
    #[case::mixed_keys_object(
        json!({"0": 1.5, "name": 2}),
        TopkValue::r#struct([("0", TopkValue::f64(1.5)), ("name", TopkValue::i64(2))])
    )]
    #[case::non_u32_index_object(
        json!({"4294967296": 1}),
        TopkValue::r#struct([("4294967296", TopkValue::i64(1))])
    )]
    #[case::nested_struct(
        json!({"a": {"b": [1, 2]}}),
        TopkValue::r#struct([(
            "a",
            TopkValue::r#struct([("b", TopkValue::list(vec![1_i64, 2]))])
        )])
    )]
    fn from_json(#[case] input: serde_json::Value, #[case] expected: TopkValue) {
        assert_eq!(TopkValue::try_from(input).unwrap(), expected);
    }

    #[rstest]
    #[case::mixed_number_string(json!([1, "a"]))]
    #[case::bool_array(json!([true, false]))]
    #[case::null_array(json!([null]))]
    #[case::object_array(json!([{}]))]
    #[case::ragged_matrix(json!([[1], [2, 3]]))]
    #[case::zero_column_matrix(json!([[]]))]
    #[case::non_numeric_matrix(json!([[1], ["a"]]))]
    #[case::mixed_matrix_scalar(json!([[1], 2]))]
    fn from_json_invalid(#[case] input: serde_json::Value) {
        assert!(TopkValue::try_from(input).is_err());
    }

    #[rstest]
    // scalars
    #[case::null(TopkValue::null(), json!(null))]
    #[case::bool(TopkValue::bool(true), json!(true))]
    #[case::string(TopkValue::string("a"), json!("a"))]
    #[case::u32(TopkValue::u32(7), json!(7))]
    #[case::u64(TopkValue::u64(i64::MAX as u64 + 1), json!(i64::MAX as u64 + 1))]
    #[case::i32(TopkValue::i32(-7), json!(-7))]
    #[case::i64(TopkValue::i64(-8), json!(-8))]
    #[case::f64(TopkValue::f64(0.5), json!(0.5))]
    // binary
    #[case::binary(TopkValue::binary(vec![1_u8, 2, 3]), json!([1, 2, 3]))]
    // lists
    #[case::u8_list(TopkValue::list(vec![1_u8, 2]), json!([1, 2]))]
    #[case::i8_list(TopkValue::list(vec![-1_i8, 2]), json!([-1, 2]))]
    #[case::u32_list(TopkValue::list(vec![1_u32, 2]), json!([1, 2]))]
    #[case::i32_list(TopkValue::list(vec![-1_i32, 2]), json!([-1, 2]))]
    #[case::u64_list(TopkValue::list(vec![1_u64, 2]), json!([1, 2]))]
    #[case::i64_list(TopkValue::list(vec![1_i64, 2]), json!([1, 2]))]
    #[case::f8_list(TopkValue::list(f8s(&[1.5, 2.5])), json!([1.5, 2.5]))]
    #[case::f16_list(TopkValue::list(f16s(&[1.5, 2.5])), json!([1.5, 2.5]))]
    #[case::f32_list(TopkValue::list(vec![1.5_f32, 2.5]), json!([1.5, 2.5]))]
    #[case::f64_list(TopkValue::list(vec![0.5_f64, 1.25]), json!([0.5, 1.25]))]
    #[case::string_list(TopkValue::list(vec!["a", "b"]), json!(["a", "b"]))]
    // sparse vectors
    #[case::f32_sparse_vector(
        TopkValue::f32_sparse_vector(vec![0, 2], vec![1.5, 3.0]),
        json!({"0": 1.5, "2": 3.0})
    )]
    #[case::f16_sparse_vector(
        TopkValue::f16_sparse_vector(vec![0, 2], f16s(&[1.5, 3.0])),
        json!({"0": 1.5, "2": 3.0})
    )]
    #[case::f8_sparse_vector(
        TopkValue::f8_sparse_vector(vec![0, 2], f8s(&[1.5, 3.0])),
        json!({"0": 1.5, "2": 3.0})
    )]
    #[case::u8_sparse_vector(
        TopkValue::u8_sparse_vector(vec![0, 2], vec![1, 3]),
        json!({"0": 1, "2": 3})
    )]
    #[case::i8_sparse_vector(
        TopkValue::i8_sparse_vector(vec![0, 2], vec![-1, 3]),
        json!({"0": -1, "2": 3})
    )]
    // matrices
    #[case::f32_matrix(
        TopkValue::matrix(2, vec![1.5_f32, 2.5, 3.5, 4.5]),
        json!([[1.5, 2.5], [3.5, 4.5]])
    )]
    #[case::f16_matrix(
        TopkValue::matrix(2, f16s(&[1.5, 2.5, 3.5, 4.5])),
        json!([[1.5, 2.5], [3.5, 4.5]])
    )]
    #[case::f8_matrix(
        TopkValue::matrix(2, f8s(&[1.5, 2.5, 3.5, 4.5])),
        json!([[1.5, 2.5], [3.5, 4.5]])
    )]
    #[case::u8_matrix(
        TopkValue::matrix(2, vec![1_u8, 2, 3, 4]),
        json!([[1, 2], [3, 4]])
    )]
    #[case::i8_matrix(
        TopkValue::matrix(2, vec![-1_i8, 2, -3, 4]),
        json!([[-1, 2], [-3, 4]])
    )]
    // structs
    #[case::struct_value(
        TopkValue::r#struct([("name", TopkValue::string("a")), ("count", TopkValue::i64(2))]),
        json!({"name": "a", "count": 2})
    )]
    #[case::empty_struct(
        TopkValue::r#struct(Vec::<(String, TopkValue)>::new()),
        json!({})
    )]
    fn to_json(#[case] input: TopkValue, #[case] expected: serde_json::Value) {
        assert_eq!(serde_json::Value::try_from(input).unwrap(), expected);
    }

    #[rstest]
    #[case::nan_f32(TopkValue::f32(f32::NAN))]
    #[case::inf_f64(TopkValue::f64(f64::INFINITY))]
    fn non_finite(#[case] input: TopkValue) {
        assert!(serde_json::Value::try_from(input.clone()).is_err());
        assert!(serde_json::to_value(Value(input)).is_err());
    }

    #[rstest]
    #[case::null(TopkValue::null(), json!(null))]
    #[case::i64_list(TopkValue::list(vec![1_i64, 2]), json!([1, 2]))]
    #[case::struct_value(
        TopkValue::r#struct([("name", TopkValue::string("a")), ("count", TopkValue::i64(2))]),
        json!({"name": "a", "count": 2})
    )]
    fn serde_roundtrip(#[case] input: TopkValue, #[case] expected: serde_json::Value) {
        let serialized = serde_json::to_value(Value(input.clone())).unwrap();
        assert_eq!(serialized, expected);
        assert_eq!(
            serde_json::from_value::<Value>(serialized)
                .unwrap()
                .into_inner(),
            input
        );
    }
}
