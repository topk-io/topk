use std::collections::HashMap;

use crate::proto::data::v1::{list, matrix, sparse_vector, value, vector, Value};

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

impl TryFrom<serde_json::Value> for Value {
    type Error = crate::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Null => Ok(Value::null()),
            serde_json::Value::Bool(value) => Ok(Value::bool(value)),
            serde_json::Value::String(value) => Ok(Value::string(value)),
            serde_json::Value::Number(value) => Ok(Value::try_from(value)?),
            serde_json::Value::Object(value) => Ok(Value::try_from(value)?),
            serde_json::Value::Array(value) => Ok(Value::try_from(value)?),
        }
    }
}

impl TryFrom<serde_json::Number> for Value {
    type Error = crate::Error;

    fn try_from(value: serde_json::Number) -> Result<Self, Self::Error> {
        if let Some(value) = value.as_i64() {
            Ok(Value::i64(value))
        } else if let Some(value) = value.as_u64() {
            Ok(Value::u64(value))
        } else {
            let value = value.as_f64().ok_or_else(|| {
                crate::Error::InvalidArgument(
                    "JSON number is outside TopK's supported numeric range".into(),
                )
            })?;

            Ok(Value::f64(value))
        }
    }
}

impl TryFrom<Vec<serde_json::Value>> for Value {
    type Error = crate::Error;

    fn try_from(values: Vec<serde_json::Value>) -> Result<Self, Self::Error> {
        let first = match values.first() {
            Some(first) => first,
            None => return Ok(Value::list(Vec::<f32>::new())),
        };

        if first.is_number() {
            return values
                .into_iter()
                .map(number_to_f32)
                .collect::<Result<Vec<_>, _>>()
                .map(Value::list);
        }

        if first.is_string() {
            return Ok(Value::list(array_of!(
                values,
                serde_json::Value::String,
                "JSON arrays must contain only numbers or strings"
            )?));
        }

        if first.is_array() {
            return Value::try_from(array_of!(
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

impl TryFrom<Vec<Vec<serde_json::Value>>> for Value {
    type Error = crate::Error;

    fn try_from(rows: Vec<Vec<serde_json::Value>>) -> Result<Self, Self::Error> {
        let num_cols = rows
            .first()
            .map(Vec::len)
            .filter(|num_cols| *num_cols > 0)
            .ok_or_else(|| {
                crate::Error::InvalidArgument("JSON matrices must have at least one column".into())
            })?;

        let mut values = Vec::with_capacity(rows.len() * num_cols);
        for row in rows {
            if row.len() != num_cols {
                return Err(crate::Error::InvalidArgument(
                    "JSON matrix rows must have the same length".into(),
                ));
            }

            for value in row {
                values.push(number_to_f32(value)?);
            }
        }

        if values.len() >= u32::MAX as usize {
            return Err(crate::Error::InvalidArgument(
                "JSON matrix has too many values".into(),
            ));
        }

        Ok(Value::matrix(num_cols as u32, values))
    }
}

impl TryFrom<serde_json::Map<String, serde_json::Value>> for Value {
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
            return Ok(Value::f32_sparse_vector(indices, values));
        }

        object
            .into_iter()
            .map(|(key, value)| Ok((key, Value::try_from(value)?)))
            .collect::<Result<HashMap<_, _>, _>>()
            .map(Value::r#struct)
    }
}

impl TryFrom<Value> for serde_json::Value {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
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

    #[rstest]
    #[case::i64(json!(42), Value::i64(42))]
    #[case::i64_negative(json!(-5), Value::i64(-5))]
    #[case::u64(
        serde_json::Value::Number(serde_json::Number::from(i64::MAX as u64 + 1)),
        Value::u64(i64::MAX as u64 + 1)
    )]
    #[case::f64(json!(0.1), Value::f64(0.1))]
    #[case::ints(json!([1, 2]), Value::list(vec![1.0_f32, 2.0]))]
    #[case::mixed(json!([1, 2.5]), Value::list(vec![1.0_f32, 2.5]))]
    #[case::empty_array(json!([]), Value::list(Vec::<f32>::new()))]
    #[case::strings(json!(["a", "b"]), Value::list(vec!["a", "b"]))]
    #[case::matrix(
        json!([[1, 2], [3.5, 4]]),
        Value::matrix(2, vec![1.0_f32, 2.0, 3.5, 4.0])
    )]
    #[case::sparse_vector(
        json!({"0": 1.5, "2": 3.0}),
        Value::f32_sparse_vector(vec![0, 2], vec![1.5, 3.0])
    )]
    #[case::struct_value(
        json!({"name": "a", "count": 2}),
        Value::r#struct([("name", Value::string("a")), ("count", Value::i64(2))])
    )]
    #[case::empty_object(
        json!({}),
        Value::r#struct(Vec::<(String, Value)>::new())
    )]
    fn from_json(#[case] input: serde_json::Value, #[case] expected: Value) {
        assert_eq!(Value::try_from(input).unwrap(), expected);
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
        assert!(Value::try_from(input).is_err());
    }

    #[rstest]
    #[case::null(Value::null(), json!(null))]
    #[case::bool(Value::bool(true), json!(true))]
    #[case::string(Value::string("a"), json!("a"))]
    #[case::u32(Value::u32(7), json!(7))]
    #[case::u64(Value::u64(i64::MAX as u64 + 1), json!(i64::MAX as u64 + 1))]
    #[case::i32(Value::i32(-7), json!(-7))]
    #[case::i64(Value::i64(-8), json!(-8))]
    #[case::f64(Value::f64(0.5), json!(0.5))]
    #[case::i64_list(Value::list(vec![1_i64, 2]), json!([1, 2]))]
    #[case::u64_list(Value::list(vec![1_u64, 2]), json!([1, 2]))]
    #[case::f32_list(Value::list(vec![1.5_f32, 2.5]), json!([1.5, 2.5]))]
    #[case::string_list(Value::list(vec!["a", "b"]), json!(["a", "b"]))]
    #[case::sparse_vector(
        Value::f32_sparse_vector(vec![0, 2], vec![1.5, 3.0]),
        json!({"0": 1.5, "2": 3.0})
    )]
    #[case::matrix(
        Value::matrix(2, vec![1.5_f32, 2.5, 3.5, 4.5]),
        json!([[1.5, 2.5], [3.5, 4.5]])
    )]
    #[case::struct_value(
        Value::r#struct([("name", Value::string("a")), ("count", Value::i64(2))]),
        json!({"name": "a", "count": 2})
    )]
    fn to_json(#[case] input: Value, #[case] expected: serde_json::Value) {
        assert_eq!(serde_json::Value::try_from(input).unwrap(), expected);
    }

    #[rstest]
    #[case::nan_f32(Value::f32(f32::NAN))]
    #[case::inf_f64(Value::f64(f64::INFINITY))]
    fn to_json_invalid(#[case] input: Value) {
        assert!(serde_json::Value::try_from(input).is_err());
    }
}
