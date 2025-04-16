use pyo3::prelude::*;

#[derive(Debug, Clone, PartialEq, FromPyObject, IntoPyObject)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl Into<topk_rs::data::scalar::Scalar> for Scalar {
    fn into(self) -> topk_rs::data::scalar::Scalar {
        match self {
            Scalar::Bool(b) => topk_rs::data::scalar::Scalar::Bool(b),
            Scalar::Int(i) => topk_rs::data::scalar::Scalar::I64(i),
            Scalar::Float(f) => topk_rs::data::scalar::Scalar::F64(f),
            Scalar::String(s) => topk_rs::data::scalar::Scalar::String(s),
        }
    }
}
