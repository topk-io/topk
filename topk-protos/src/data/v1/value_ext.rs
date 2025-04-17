use super::*;

impl Value {
    pub fn null() -> Self {
        Value {
            value: Some(value::Value::Null(Null {})),
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

    pub fn float_vector(values: Vec<f32>) -> Self {
        Value {
            value: Some(value::Value::Vector(Vector {
                vector: Some(vector::Vector::Float(vector::Float { values })),
            })),
        }
    }

    pub fn bytes(value: Vec<u8>) -> Self {
        Value {
            value: Some(value::Value::Binary(value)),
        }
    }

    pub fn as_float_vector(&self) -> Option<&[f32]> {
        match &self.value {
            Some(value::Value::Vector(vec)) => match &vec.vector {
                Some(vector::Vector::Float(vector::Float { values })) => Some(values),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn byte_vector(values: Vec<u8>) -> Self {
        Value {
            value: Some(value::Value::Vector(Vector {
                vector: Some(vector::Vector::Byte(vector::Byte { values })),
            })),
        }
    }

    pub fn as_byte_vector(&self) -> Option<&[u8]> {
        match &self.value {
            Some(value::Value::Vector(vec)) => match &vec.vector {
                Some(vector::Vector::Byte(vector::Byte { values })) => Some(values),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn binary(value: Vec<u8>) -> Self {
        Value {
            value: Some(value::Value::Binary(value)),
        }
    }

    pub fn as_binary(&self) -> Option<&[u8]> {
        match &self.value {
            Some(value::Value::Binary(value)) => Some(value),
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
            value::Value::Vector(vec) => match &vec.vector {
                Some(vector::Vector::Float(v)) => format!("f32_vector({})", v.values.len()),
                Some(vector::Vector::Byte(v)) => format!("u8_vector({})", v.values.len()),
                _ => "null_vector".to_string(),
            },
            value::Value::Null(_) => "null".to_string(),
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::string(value)
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

impl From<Vec<f32>> for Value {
    fn from(value: Vec<f32>) -> Self {
        Value::float_vector(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::bool(value)
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Value::binary(value)
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

impl Vector {
    pub fn float(values: Vec<f32>) -> Self {
        Vector {
            vector: Some(vector::Vector::Float(vector::Float { values })),
        }
    }

    pub fn byte(values: Vec<u8>) -> Self {
        Vector {
            vector: Some(vector::Vector::Byte(vector::Byte { values })),
        }
    }

    pub fn len(&self) -> Option<usize> {
        match &self.vector {
            Some(vector::Vector::Float(vector::Float { values })) => Some(values.len()),
            Some(vector::Vector::Byte(vector::Byte { values })) => Some(values.len()),
            _ => None,
        }
    }
}
