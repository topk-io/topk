use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyNone, PyString},
    IntoPyObjectExt,
};

use crate::data::vector::F32Vector;

use super::vector::{F32SparseVector, SparseVector, Vector};

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Null(),
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Vector(Vector),
    SparseVector(SparseVector),
    Bytes(Vec<u8>),
}

#[pymethods]
impl Value {
    fn __str__(&self) -> String {
        match self {
            Value::Null() => "Null".to_string(),
            Value::String(s) => s.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Vector(v) => format!("Vector({:?})", v),
            Value::SparseVector(v) => format!("SparseVector({:?})", v),
            Value::Bytes(b) => format!("Bytes({:?})", b),
        }
    }
}

pub struct RawValue(pub Value);

impl<'py> FromPyObject<'py> for RawValue {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        // NOTE: it's safe to use `downcast` for custom types
        if let Ok(v) = obj.downcast::<Value>() {
            Ok(RawValue(v.get().clone()))
        } else if let Ok(v) = obj.downcast::<Vector>() {
            Ok(RawValue(Value::Vector(v.get().clone())))
        } else if let Ok(v) = obj.downcast::<SparseVector>() {
            Ok(RawValue(Value::SparseVector(v.get().clone())))
        } else if let Ok(s) = obj.downcast_exact::<PyString>() {
            Ok(RawValue(Value::String(s.extract()?)))
        } else if let Ok(i) = obj.downcast_exact::<PyInt>() {
            Ok(RawValue(Value::Int(i.extract()?)))
        } else if let Ok(b) = obj.downcast_exact::<PyBytes>() {
            Ok(RawValue(Value::Bytes(b.extract()?)))
        } else if let Ok(f) = obj.downcast_exact::<PyFloat>() {
            Ok(RawValue(Value::Float(f.extract()?)))
        } else if let Ok(b) = obj.downcast_exact::<PyBool>() {
            Ok(RawValue(Value::Bool(b.extract()?)))
        } else if let Ok(v) = F32SparseVector::extract_bound(obj) {
            Ok(RawValue(Value::SparseVector(SparseVector::F32 {
                indices: v.indices,
                values: v.values,
            })))
        } else if let Ok(v) = F32Vector::extract_bound(obj) {
            Ok(RawValue(Value::Vector(Vector::F32(v.values))))
        } else if let Ok(_) = obj.downcast_exact::<PyNone>() {
            Ok(RawValue(Value::Null()))
        } else {
            Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to Value",
                obj.get_type().name()
            )))
        }
    }
}

impl<'py> IntoPyObject<'py> for RawValue {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        match self.0 {
            Value::Null() => Ok(py.None().into_bound(py)),
            Value::String(s) => Ok(s.into_py_any(py)?.into_bound(py)),
            Value::Int(i) => Ok(i.into_py_any(py)?.into_bound(py)),
            Value::Float(f) => Ok(f.into_py_any(py)?.into_bound(py)),
            Value::Bool(b) => Ok(b.into_py_any(py)?.into_bound(py)),
            Value::Bytes(b) => Ok(b.into_py_any(py)?.into_bound(py)),
            Value::Vector(v) => Ok(match v {
                Vector::F32(v) => {
                    let list = PyList::empty(py);
                    for value in v {
                        list.append(value.into_py_any(py)?)?;
                    }
                    list.into_py_any(py)?.into_bound(py)
                }
                Vector::U8(v) => {
                    let list = PyList::empty(py);
                    for value in v {
                        list.append(value.into_py_any(py)?)?;
                    }
                    list.into_py_any(py)?.into_bound(py)
                }
            }),
            Value::SparseVector(v) => Ok(match v {
                SparseVector::F32 { indices, values } => {
                    let dict = PyDict::new(py);
                    for (i, v) in indices.iter().zip(values.iter()) {
                        dict.set_item(i.into_py_any(py)?, v.into_py_any(py)?)?;
                    }
                    dict.into_py_any(py)?.into_bound(py)
                }
                SparseVector::U8 { indices, values } => {
                    let dict = PyDict::new(py);
                    for (i, v) in indices.iter().zip(values.iter()) {
                        dict.set_item(i.into_py_any(py)?, v.into_py_any(py)?)?;
                    }
                    dict.into_py_any(py)?.into_bound(py)
                }
            }),
        }
    }
}
impl From<topk_rs::proto::v1::data::Value> for Value {
    fn from(value: topk_rs::proto::v1::data::Value) -> Self {
        match value.value {
            Some(topk_rs::proto::v1::data::value::Value::String(s)) => Value::String(s),
            Some(topk_rs::proto::v1::data::value::Value::U32(i)) => Value::Int(i as i64),
            Some(topk_rs::proto::v1::data::value::Value::U64(i)) => Value::Int(i as i64),
            Some(topk_rs::proto::v1::data::value::Value::I64(i)) => Value::Int(i as i64),
            Some(topk_rs::proto::v1::data::value::Value::I32(i)) => Value::Int(i as i64),
            Some(topk_rs::proto::v1::data::value::Value::F32(f)) => Value::Float(f as f64),
            Some(topk_rs::proto::v1::data::value::Value::F64(f)) => Value::Float(f),
            Some(topk_rs::proto::v1::data::value::Value::Bool(b)) => Value::Bool(b),
            Some(topk_rs::proto::v1::data::value::Value::Null(_)) => Value::Null(),
            Some(topk_rs::proto::v1::data::value::Value::Binary(b)) => Value::Bytes(b),
            Some(topk_rs::proto::v1::data::value::Value::Vector(v)) => match v.vector {
                Some(topk_rs::proto::v1::data::vector::Vector::Float(v)) => {
                    Value::Vector(Vector::F32(v.values))
                }
                Some(topk_rs::proto::v1::data::vector::Vector::Byte(v)) => {
                    Value::Vector(Vector::U8(v.values))
                }
                t => unreachable!("Unknown vector type: {:?}", t),
            },
            Some(topk_rs::proto::v1::data::value::Value::SparseVector(sv)) => {
                Value::SparseVector(match sv.values {
                    Some(topk_rs::proto::v1::data::sparse_vector::Values::F32(v)) => {
                        SparseVector::F32 {
                            indices: sv.indices,
                            values: v.values,
                        }
                    }
                    Some(topk_rs::proto::v1::data::sparse_vector::Values::U8(v)) => {
                        SparseVector::U8 {
                            indices: sv.indices,
                            values: v.values,
                        }
                    }
                    t => unreachable!("Unknown sparse vector type: {:?}", t),
                })
            }
            None => Value::Null(),
        }
    }
}

impl From<Value> for topk_rs::proto::v1::data::Value {
    fn from(value: Value) -> Self {
        match value {
            Value::Bool(b) => topk_rs::proto::v1::data::Value::bool(b),
            Value::Int(i) => topk_rs::proto::v1::data::Value::i64(i),
            Value::Float(f) => topk_rs::proto::v1::data::Value::f64(f),
            Value::String(s) => topk_rs::proto::v1::data::Value::string(s),
            Value::Null() => topk_rs::proto::v1::data::Value::null(),
            Value::Bytes(b) => topk_rs::proto::v1::data::Value::binary(b),
            Value::Vector(v) => match v {
                Vector::F32(v) => topk_rs::proto::v1::data::Value::f32_vector(v),
                Vector::U8(v) => topk_rs::proto::v1::data::Value::u8_vector(v),
            },
            Value::SparseVector(v) => match v {
                SparseVector::F32 { indices, values } => {
                    topk_rs::proto::v1::data::Value::f32_sparse_vector(indices, values)
                }
                SparseVector::U8 { indices, values } => {
                    topk_rs::proto::v1::data::Value::u8_sparse_vector(indices, values)
                }
            },
        }
    }
}

impl From<RawValue> for topk_rs::proto::v1::data::Value {
    fn from(value: RawValue) -> Self {
        value.0.into()
    }
}
