use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyNone, PyString},
    IntoPyObjectExt,
};

use super::vector::{SparseVector, Vector};

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

pub struct RawValue(pub Value);

impl<'py> FromPyObject<'py> for RawValue {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        // NOTE: it's safe to use `downcast` for `Value` since it's a custom type
        if let Ok(v) = obj.downcast::<Value>() {
            Ok(RawValue(v.get().clone()))
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
        } else if let Ok(v) = obj.downcast_exact::<PyList>() {
            // Try converting to vector from starting with most restrictive type first.
            if let Ok(values) = v.extract::<Vec<f32>>() {
                Ok(RawValue(Value::Vector(Vector::F32(values))))
            } else {
                Err(PyTypeError::new_err(format!(
                    "Can't convert from {:?} to Value",
                    obj.get_type().name()
                )))
            }
        } else if let Ok(d) = obj.downcast_exact::<PyDict>() {
            if let Ok(indices) = d.keys().extract::<Vec<u32>>() {
                let values = d.values();
                if let Ok(values) = values.extract::<Vec<f32>>() {
                    Ok(RawValue(Value::SparseVector(SparseVector::F32 {
                        indices,
                        values,
                    })))
                } else if let Ok(values) = values.extract::<Vec<u8>>() {
                    Ok(RawValue(Value::SparseVector(SparseVector::U8 {
                        indices,
                        values,
                    })))
                } else {
                    Err(PyTypeError::new_err(format!(
                        "Can't convert from {:?} to Value",
                        obj.get_type().name()
                    )))
                }
            } else {
                Err(PyTypeError::new_err(format!(
                    "Can't convert from {:?} to Value",
                    obj.get_type().name()
                )))
            }
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
impl From<topk_protos::v1::data::Value> for Value {
    fn from(value: topk_protos::v1::data::Value) -> Self {
        match value.value {
            Some(topk_protos::v1::data::value::Value::String(s)) => Value::String(s),
            Some(topk_protos::v1::data::value::Value::U32(i)) => Value::Int(i as i64),
            Some(topk_protos::v1::data::value::Value::U64(i)) => Value::Int(i as i64),
            Some(topk_protos::v1::data::value::Value::I64(i)) => Value::Int(i as i64),
            Some(topk_protos::v1::data::value::Value::I32(i)) => Value::Int(i as i64),
            Some(topk_protos::v1::data::value::Value::F32(f)) => Value::Float(f as f64),
            Some(topk_protos::v1::data::value::Value::F64(f)) => Value::Float(f),
            Some(topk_protos::v1::data::value::Value::Bool(b)) => Value::Bool(b),
            Some(topk_protos::v1::data::value::Value::Null(_)) => Value::Null(),
            Some(topk_protos::v1::data::value::Value::Binary(b)) => Value::Bytes(b),
            Some(topk_protos::v1::data::value::Value::Vector(v)) => match v.vector {
                Some(topk_protos::v1::data::vector::Vector::Float(v)) => {
                    Value::Vector(Vector::F32(v.values))
                }
                Some(topk_protos::v1::data::vector::Vector::Byte(v)) => {
                    Value::Vector(Vector::U8(v.values))
                }
                t => unreachable!("Unknown vector type: {:?}", t),
            },
            Some(topk_protos::v1::data::value::Value::SparseVector(v)) => match v.values {
                Some(topk_protos::v1::data::sparse_vector::Values::F32(v)) => {
                    Value::Vector(Vector::F32(v.values))
                }
                Some(topk_protos::v1::data::sparse_vector::Values::U8(v)) => {
                    Value::Vector(Vector::U8(v.values))
                }
                t => unreachable!("Unknown sparse vector type: {:?}", t),
            },
            None => Value::Null(),
        }
    }
}

impl From<Value> for topk_protos::v1::data::Value {
    fn from(value: Value) -> Self {
        Self {
            value: Some(match value {
                Value::Bool(b) => topk_protos::v1::data::value::Value::Bool(b),
                Value::Int(i) => topk_protos::v1::data::value::Value::I64(i),
                Value::Float(f) => topk_protos::v1::data::value::Value::F64(f),
                Value::String(s) => topk_protos::v1::data::value::Value::String(s),
                Value::Null() => {
                    topk_protos::v1::data::value::Value::Null(topk_protos::v1::data::Null {})
                }
                Value::Bytes(b) => topk_protos::v1::data::value::Value::Binary(b),
                Value::Vector(v) => match v {
                    Vector::F32(v) => topk_protos::v1::data::value::Value::Vector(
                        topk_protos::v1::data::Vector::float(v),
                    ),
                    Vector::U8(v) => topk_protos::v1::data::value::Value::Vector(
                        topk_protos::v1::data::Vector::byte(v),
                    ),
                },
                Value::SparseVector(v) => match v {
                    SparseVector::F32 { indices, values } => {
                        topk_protos::v1::data::value::Value::SparseVector(
                            topk_protos::v1::data::SparseVector::f32(indices, values),
                        )
                    }
                    SparseVector::U8 { indices, values } => {
                        topk_protos::v1::data::value::Value::SparseVector(
                            topk_protos::v1::data::SparseVector::u8(indices, values),
                        )
                    }
                },
            }),
        }
    }
}

impl From<RawValue> for topk_protos::v1::data::Value {
    fn from(value: RawValue) -> Self {
        value.0.into()
    }
}
