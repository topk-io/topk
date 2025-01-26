use pyo3::prelude::*;

#[derive(Debug, Clone, PartialEq, FromPyObject, IntoPyObject)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl Into<topk_protos::v1::data::Value> for Scalar {
    fn into(self) -> topk_protos::v1::data::Value {
        topk_protos::v1::data::Value {
            value: Some(match self {
                Scalar::Bool(b) => topk_protos::v1::data::value::Value::Bool(b),
                Scalar::Int(i) => topk_protos::v1::data::value::Value::I64(i),
                Scalar::Float(f) => topk_protos::v1::data::value::Value::F64(f),
                Scalar::String(s) => topk_protos::v1::data::value::Value::String(s),
            }),
        }
    }
}
