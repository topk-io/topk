use bytemuck::{cast_slice, cast_vec};
use bytes::Bytes;
use std::collections::HashMap;

use crate::proto::data::v1::{
    data_ext::IntoListValues, list, sparse_vector, value, vector, List, Null, SparseVector, Struct,
    Value,
};

impl Value {
    pub fn null() -> Self {
        Value {
            value: Some(value::Value::Null(Null {})),
        }
    }

    pub fn as_null(&self) -> Option<()> {
        match &self.value {
            Some(value::Value::Null(_)) => Some(()),
            _ => None,
        }
    }

    pub fn bool(value: bool) -> Self {
        Value {
            value: Some(value::Value::Bool(value)),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            Some(value::Value::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn string(value: impl Into<String>) -> Self {
        Value {
            value: Some(value::Value::String(value.into())),
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match &self.value {
            Some(value::Value::String(value)) => Some(value),
            _ => None,
        }
    }

    pub fn u32(value: u32) -> Self {
        Value {
            value: Some(value::Value::U32(value)),
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match &self.value {
            Some(value::Value::U32(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn u64(value: u64) -> Self {
        Value {
            value: Some(value::Value::U64(value)),
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match &self.value {
            Some(value::Value::U64(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn i32(value: i32) -> Self {
        Value {
            value: Some(value::Value::I32(value)),
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match &self.value {
            Some(value::Value::I32(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn i64(value: i64) -> Self {
        Value {
            value: Some(value::Value::I64(value)),
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match &self.value {
            Some(value::Value::I64(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn f32(value: f32) -> Self {
        Value {
            value: Some(value::Value::F32(value)),
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match &self.value {
            Some(value::Value::F32(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn f64(value: f64) -> Self {
        Value {
            value: Some(value::Value::F64(value)),
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match &self.value {
            Some(value::Value::F64(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn f32_sparse_vector(indices: Vec<u32>, values: Vec<f32>) -> Self {
        Value {
            value: Some(value::Value::SparseVector(SparseVector {
                indices,
                values: Some(sparse_vector::Values::F32(sparse_vector::F32Values {
                    values,
                })),
            })),
        }
    }

    pub fn u8_sparse_vector(indices: Vec<u32>, values: Vec<u8>) -> Self {
        Value {
            value: Some(value::Value::SparseVector(SparseVector {
                indices,
                values: Some(sparse_vector::Values::U8(sparse_vector::U8Values {
                    values,
                })),
            })),
        }
    }

    /// Alias for `binary`
    pub fn bytes(value: impl Into<Bytes>) -> Self {
        Value::binary(value)
    }

    pub fn binary(value: impl Into<Bytes>) -> Self {
        Value {
            value: Some(value::Value::Binary(value.into())),
        }
    }

    pub fn as_binary(&self) -> Option<&[u8]> {
        match &self.value {
            Some(value::Value::Binary(value)) => Some(value),
            _ => None,
        }
    }

    /// Create a struct value from a map of values.
    pub fn r#struct<K: Into<String>>(values: impl IntoIterator<Item = (K, Value)>) -> Self {
        Value {
            value: Some(value::Value::Struct(Struct {
                fields: values.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            })),
        }
    }

    pub fn as_struct(&self) -> Option<&HashMap<String, Value>> {
        match &self.value {
            Some(value::Value::Struct(s)) => Some(&s.fields),
            _ => None,
        }
    }

    /// Create a list value from a vector of values.
    pub fn list<T: IntoListValues>(values: T) -> Self {
        Value {
            value: Some(value::Value::List(List {
                values: Some(values.into_list_values()),
            })),
        }
    }

    pub fn as_u8_list(&self) -> Option<&[u8]> {
        match &self.value {
            Some(value::Value::List(list)) => match &list.values {
                Some(list::Values::U8(v)) => Some(&v.values),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_i8_list(&self) -> Option<&[i8]> {
        match &self.value {
            Some(value::Value::List(list)) => match &list.values {
                Some(list::Values::I8(v)) => Some(&v.as_ref()),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_f32_list(&self) -> Option<&[f32]> {
        match &self.value {
            Some(value::Value::List(list)) => match &list.values {
                Some(list::Values::F32(v)) => Some(&v.values),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_string_list(&self) -> Option<&[String]> {
        match &self.value {
            Some(value::Value::List(list)) => match &list.values {
                Some(list::Values::String(v)) => Some(&v.values),
                _ => None,
            },
            _ => None,
        }
    }
}

impl value::Value {
    pub fn to_user_friendly_type_name(&self) -> String {
        match self {
            value::Value::Bool(_) => "bool".to_string(),
            value::Value::U32(_) => "u32".to_string(),
            value::Value::U64(_) => "u64".to_string(),
            value::Value::I32(_) => "i32".to_string(),
            value::Value::I64(_) => "i64".to_string(),
            value::Value::F32(_) => "f32".to_string(),
            value::Value::F64(_) => "f64".to_string(),
            value::Value::String(_) => "string".to_string(),
            value::Value::Binary(v) => {
                format!("binary({})", v.len())
            }
            #[allow(deprecated)]
            value::Value::Vector(vec) => match &vec.vector {
                Some(vector::Vector::Float(v)) => format!("f32_vector({})", v.values.len()),
                Some(vector::Vector::Byte(v)) => format!("u8_vector({})", v.values.len()),
                _ => "null_vector".to_string(),
            },
            value::Value::SparseVector(v) => match &v.values {
                Some(sparse_vector::Values::F32(_)) => "f32_sparse_vector".to_string(),
                Some(sparse_vector::Values::U8(_)) => "u8_sparse_vector".to_string(),
                _ => "null_sparse_vector".to_string(),
            },
            value::Value::List(v) => match &v.values {
                Some(list::Values::U32(_)) => "list<u32>".to_string(),
                Some(list::Values::U64(_)) => "list<u64>".to_string(),
                Some(list::Values::I32(_)) => "list<i32>".to_string(),
                Some(list::Values::I64(_)) => "list<i64>".to_string(),
                Some(list::Values::F32(_)) => "list<f32>".to_string(),
                Some(list::Values::F64(_)) => "list<f64>".to_string(),
                Some(list::Values::String(_)) => "list<string>".to_string(),
                Some(list::Values::U8(_)) => "list<u8>".to_string(),
                Some(list::Values::I8(_)) => "list<i8>".to_string(),
                _ => "null_list".to_string(),
            },
            value::Value::Struct(_) => "struct<string, Value>".to_string(),
            value::Value::Null(_) => "null".to_string(),
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::bool(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::string(value)
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::null()
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::string(value.to_string())
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::u32(value)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value::u64(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::i32(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::i64(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::f32(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::f64(value)
    }
}

impl From<Vec<u32>> for Value {
    fn from(value: Vec<u32>) -> Self {
        Value::list(value)
    }
}

impl From<Vec<u64>> for Value {
    fn from(value: Vec<u64>) -> Self {
        Value::list(value)
    }
}

impl From<Vec<f32>> for Value {
    fn from(value: Vec<f32>) -> Self {
        Value::list(value)
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Value::list(value)
    }
}

impl From<Vec<i8>> for Value {
    fn from(value: Vec<i8>) -> Self {
        Value::list(value)
    }
}

impl From<Vec<&str>> for Value {
    fn from(value: Vec<&str>) -> Self {
        Value::list(
            value
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
    }
}

impl From<Vec<String>> for Value {
    fn from(value: Vec<String>) -> Self {
        Value::list(value)
    }
}

impl From<SparseVector> for Value {
    fn from(value: SparseVector) -> Self {
        Value {
            value: Some(value::Value::SparseVector(value)),
        }
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(value: HashMap<String, Value>) -> Self {
        Value::r#struct(value)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.into(),
            None => Value::null(),
        }
    }
}

impl From<list::I8> for Vec<i8> {
    fn from(v: list::I8) -> Self {
        cast_vec(v.values)
    }
}

impl AsRef<[i8]> for list::I8 {
    fn as_ref(&self) -> &[i8] {
        cast_slice(&self.values)
    }
}

impl Struct {
    pub fn depth(&self) -> usize {
        let mut depth = 1;
        for (_, value) in &self.fields {
            if let Some(value::Value::Struct(s)) = &value.value {
                depth = s.depth() + 1;
            }
        }
        depth
    }
}
