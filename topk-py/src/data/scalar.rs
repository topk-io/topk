use pyo3::prelude::*;

#[derive(Debug, Clone, PartialEq, FromPyObject, IntoPyObject)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl Into<topk_rs::data::Scalar> for Scalar {
    fn into(self) -> topk_rs::data::Scalar {
        match self {
            Scalar::Bool(b) => topk_rs::data::Scalar::Bool(b),
            Scalar::Int(i) => topk_rs::data::Scalar::I64(i),
            Scalar::Float(f) => topk_rs::data::Scalar::F64(f),
            Scalar::String(s) => topk_rs::data::Scalar::String(s),
        }
    }
}
