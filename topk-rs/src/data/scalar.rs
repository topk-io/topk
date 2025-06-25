#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Bool(bool),
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
}

impl From<bool> for Scalar {
    fn from(value: bool) -> Self {
        Scalar::Bool(value)
    }
}

impl From<u32> for Scalar {
    fn from(value: u32) -> Self {
        Scalar::U32(value)
    }
}

impl From<u64> for Scalar {
    fn from(value: u64) -> Self {
        Scalar::U64(value)
    }
}

impl From<i32> for Scalar {
    fn from(value: i32) -> Self {
        Scalar::I32(value)
    }
}

impl From<i64> for Scalar {
    fn from(value: i64) -> Self {
        Scalar::I64(value)
    }
}

impl From<f32> for Scalar {
    fn from(value: f32) -> Self {
        Scalar::F32(value)
    }
}

impl From<f64> for Scalar {
    fn from(value: f64) -> Self {
        Scalar::F64(value)
    }
}

impl Into<crate::proto::v1::data::Value> for Scalar {
    fn into(self) -> crate::proto::v1::data::Value {
        crate::proto::v1::data::Value {
            value: Some(match self {
                Scalar::Bool(b) => crate::proto::v1::data::value::Value::Bool(b),
                Scalar::I32(i) => crate::proto::v1::data::value::Value::I32(i),
                Scalar::I64(i) => crate::proto::v1::data::value::Value::I64(i),
                Scalar::F32(f) => crate::proto::v1::data::value::Value::F32(f),
                Scalar::F64(f) => crate::proto::v1::data::value::Value::F64(f),
                Scalar::U32(u) => crate::proto::v1::data::value::Value::U32(u),
                Scalar::U64(u) => crate::proto::v1::data::value::Value::U64(u),
                Scalar::String(s) => crate::proto::v1::data::value::Value::String(s),
            }),
        }
    }
}
