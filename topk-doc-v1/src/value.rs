use std::collections::HashMap;

use rkyv::{Archive, Deserialize, Serialize, rancor::Error as RkyvError};

use crate::{ListValue, ScalarType};

#[derive(Archive, Deserialize, Serialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct StructValue {
    pub fields: HashMap<String, Value>,
}

#[derive(Archive, Deserialize, Serialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct SparseVector {
    pub indices: Vec<u32>,
    pub values: ListValue,
}

impl SparseVector {
    pub fn f32(indices: impl Into<Vec<u32>>, values: Vec<f32>) -> Self {
        Self {
            indices: indices.into(),
            values: values.into(),
        }
    }

    pub fn u8(indices: impl Into<Vec<u32>>, values: Vec<u8>) -> Self {
        Self {
            indices: indices.into(),
            values: values.into(),
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Clone, Debug, PartialEq)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator,
    __S::Error: rkyv::rancor::Source,
))]
#[rkyv(deserialize_bounds(__D::Error: rkyv::rancor::Source))]
#[rkyv(bytecheck(bounds(__C: rkyv::validation::ArchiveContext)))]
#[repr(C, u8)]
pub enum Value {
    Null,
    // Boolean
    Bool(bool),
    // Unsigned
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    // Signed
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    // Floating point
    F32(f32),
    F64(f64),
    // String
    String(String),
    // Binary
    Binary(Vec<u8>),
    // Sparse vector
    SparseVector(SparseVector),
    // List
    List(ListValue),
    // Struct
    Struct(#[rkyv(omit_bounds)] StructValue),
}

impl Value {
    #[inline(always)]
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(rkyv::to_bytes::<RkyvError>(self)?.to_vec())
    }

    #[inline(always)]
    pub fn decode(data: &[u8]) -> anyhow::Result<Value> {
        Ok(rkyv::from_bytes::<_, RkyvError>(data)?)
    }

    #[inline(always)]
    pub fn access<'a>(data: &'a [u8]) -> anyhow::Result<&'a ArchivedValue> {
        Ok(rkyv::access::<ArchivedValue, RkyvError>(data)?)
    }

    // Constructors

    pub fn null() -> Self {
        Value::Null
    }

    pub fn bool(value: bool) -> Self {
        Value::Bool(value)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn u8(value: u8) -> Self {
        Value::U8(value)
    }

    pub fn as_u8(&self) -> Option<u8> {
        match self {
            Value::U8(value) => Some(*value),
            _ => None,
        }
    }

    pub fn u16(value: u16) -> Self {
        Value::U16(value)
    }

    pub fn as_u16(&self) -> Option<u16> {
        match self {
            Value::U16(value) => Some(*value),
            _ => None,
        }
    }

    pub fn u32(value: u32) -> Self {
        Value::U32(value)
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Value::U32(value) => Some(*value),
            _ => None,
        }
    }

    pub fn u64(value: u64) -> Self {
        Value::U64(value)
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::U64(value) => Some(*value),
            _ => None,
        }
    }

    pub fn i8(value: i8) -> Self {
        Value::I8(value)
    }

    pub fn as_i8(&self) -> Option<i8> {
        match self {
            Value::I8(value) => Some(*value),
            _ => None,
        }
    }

    pub fn i16(value: i16) -> Self {
        Value::I16(value)
    }

    pub fn as_i16(&self) -> Option<i16> {
        match self {
            Value::I16(value) => Some(*value),
            _ => None,
        }
    }

    pub fn i32(value: i32) -> Self {
        Value::I32(value)
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Value::I32(value) => Some(*value),
            _ => None,
        }
    }

    pub fn i64(value: i64) -> Self {
        Value::I64(value)
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(value) => Some(*value),
            _ => None,
        }
    }

    pub fn f32(value: f32) -> Self {
        Value::F32(value)
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::F32(value) => Some(*value),
            _ => None,
        }
    }

    pub fn f64(value: f64) -> Self {
        Value::F64(value)
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F64(value) => Some(*value),
            _ => None,
        }
    }

    pub fn string(value: impl Into<String>) -> Self {
        Value::String(value.into())
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn binary(value: impl Into<Vec<u8>>) -> Self {
        Value::Binary(value.into())
    }

    pub fn list<T: ScalarType>(value: impl Into<Vec<T>>) -> Self {
        Value::List(value.into().into())
    }

    pub fn r#struct<K: Into<String>>(values: impl IntoIterator<Item = (K, Value)>) -> Self {
        Value::Struct(StructValue {
            fields: values.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        })
    }

    pub fn sparse_vector(indices: impl Into<Vec<u32>>, values: impl Into<ListValue>) -> Self {
        Value::SparseVector(SparseVector {
            indices: indices.into(),
            values: values.into(),
        })
    }
}

// Scalar values

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_string())
    }
}

impl<T> From<T> for Value
where
    T: ScalarType,
{
    fn from(value: T) -> Self {
        T::to_value(value)
    }
}

// Scalar lists

impl<T> From<Vec<T>> for Value
where
    T: ScalarType,
{
    fn from(value: Vec<T>) -> Self {
        Value::List(value.into())
    }
}
