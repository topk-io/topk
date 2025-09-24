use pyo3::prelude::*;

use crate::data::list::{List, Values};

#[derive(Debug, Clone, PartialEq, FromPyObject, IntoPyObject)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(List),
}

impl From<Scalar> for topk_rs::proto::v1::data::Value {
    fn from(scalar: Scalar) -> Self {
        match scalar {
            Scalar::Bool(b) => topk_rs::proto::v1::data::Value::bool(b),
            Scalar::Int(i) => topk_rs::proto::v1::data::Value::i64(i),
            Scalar::Float(f) => topk_rs::proto::v1::data::Value::f64(f),
            Scalar::String(s) => topk_rs::proto::v1::data::Value::string(s),
            Scalar::List(l) => match l.values {
                Values::U8(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::U32(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::U64(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::I8(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::I32(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::I64(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::F32(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::F64(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::String(values) => topk_rs::proto::v1::data::Value::list(values),
            },
        }
    }
}
