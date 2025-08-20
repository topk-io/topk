use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyBool, PyFloat, PyInt, PyString},
};
use crate::data::list::{List, Values};

#[derive(Debug, Clone, PartialEq, IntoPyObject)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(List),
}

/// Custom FromPyObject impl is needed to extract Vec<{String,i64,f32}> into List
///  so that the Scalar is normalised
impl<'py> FromPyObject<'py> for Scalar {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        // Non native types, created by data constructors
        if let Ok(v) = obj.downcast::<List>() {
            Ok(Scalar::List(v.borrow().clone()))
        // Native list types
        } else if let Ok(v) = obj.extract::<Vec<i64>>() {
            Ok(Scalar::List(List {
                values: Values::I64(v),
            }))
        } else if let Ok(v) = obj.extract::<Vec<f32>>() {
            Ok(Scalar::List(List {
                values: Values::F32(v),
            }))
        } else if let Ok(v) = obj.extract::<Vec<String>>() {
            Ok(Scalar::List(List {
                values: Values::String(v),
            }))
        // Native primitive types
        } else if let Ok(s) = obj.downcast_exact::<PyString>() {
            Ok(Scalar::String(s.extract()?))
        } else if let Ok(i) = obj.downcast_exact::<PyInt>() {
            Ok(Scalar::Int(i.extract()?))
        } else if let Ok(f) = obj.downcast_exact::<PyFloat>() {
            Ok(Scalar::Float(f.extract()?))
        } else if let Ok(b) = obj.downcast_exact::<PyBool>() {
            Ok(Scalar::Bool(b.extract()?))
        } else {
            Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to Scalar",
                obj.get_type().name()
            )))
        }
    }
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
                Values::I32(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::I64(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::F32(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::F64(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::String(values) => topk_rs::proto::v1::data::Value::list(values),
            },
        }
    }
}
