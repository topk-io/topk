#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
}

impl From<i64> for Scalar {
    fn from(value: i64) -> Self {
        Scalar::I64(value)
    }
}

impl From<f64> for Scalar {
    fn from(value: f64) -> Self {
        Scalar::F64(value)
    }
}

impl From<bool> for Scalar {
    fn from(value: bool) -> Self {
        Scalar::Bool(value)
    }
}

impl From<String> for Scalar {
    fn from(value: String) -> Self {
        Scalar::String(value)
    }
}

impl Into<topk_rs::data::Scalar> for Scalar {
    fn into(self) -> topk_rs::data::Scalar {
        match self {
            Scalar::Bool(b) => topk_rs::data::Scalar::Bool(b),
            Scalar::I64(i) => topk_rs::data::Scalar::I64(i),
            Scalar::F64(f) => topk_rs::data::Scalar::F64(f),
            Scalar::String(s) => topk_rs::data::Scalar::String(s),
        }
    }
}

impl Into<topk_rs::proto::v1::data::Value> for Scalar {
    fn into(self) -> topk_rs::proto::v1::data::Value {
        match self {
            Scalar::Bool(b) => topk_rs::proto::v1::data::Value::bool(b),
            Scalar::I64(i) => topk_rs::proto::v1::data::Value::i64(i),
            Scalar::F64(f) => topk_rs::proto::v1::data::Value::f64(f),
            Scalar::String(s) => topk_rs::proto::v1::data::Value::string(s),
        }
    }
}
