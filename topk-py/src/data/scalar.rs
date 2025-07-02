use pyo3::prelude::*;

#[derive(Debug, Clone, PartialEq, FromPyObject, IntoPyObject)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl From<Scalar> for topk_rs::proto::v1::data::Value {
    fn from(scalar: Scalar) -> Self {
        match scalar {
            Scalar::Bool(b) => topk_rs::proto::v1::data::Value::bool(b),
            Scalar::Int(i) => topk_rs::proto::v1::data::Value::i64(i),
            Scalar::Float(f) => topk_rs::proto::v1::data::Value::f64(f),
            Scalar::String(s) => topk_rs::proto::v1::data::Value::string(s),
        }
    }
}
