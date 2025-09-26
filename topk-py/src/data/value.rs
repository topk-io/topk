use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyNone, PyString},
    IntoPyObjectExt,
};

use crate::data::{
    list::{List, Values},
    vector::F32SparseVector,
};

use super::vector::SparseVector;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Null(),
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    SparseVector(SparseVector),
    Bytes(Vec<u8>),
    List(List),
}

impl<'py> FromPyObject<'py> for Value {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        // NOTE: it's safe to use `downcast` for custom types

        if let Ok(v) = obj.downcast::<List>() {
            Ok(Value::List(v.borrow().clone()))
        // PyBytes can be extracted as Vec<f32> so it needs to be handled before list(f32)
        } else if let Ok(b) = obj.downcast_exact::<PyBytes>() {
            Ok(Value::Bytes(b.extract()?))
        } else if let Ok(v) = obj.extract::<Vec<f32>>() {
            Ok(Value::List(List {
                values: Values::F32(v),
            }))
        } else if let Ok(v) = obj.extract::<Vec<String>>() {
            Ok(Value::List(List {
                values: Values::String(v),
            }))
        } else if let Ok(v) = obj.downcast::<SparseVector>() {
            Ok(Value::SparseVector(v.get().clone()))
        } else if let Ok(s) = obj.downcast_exact::<PyString>() {
            Ok(Value::String(s.extract()?))
        } else if let Ok(i) = obj.downcast_exact::<PyInt>() {
            Ok(Value::Int(i.extract()?))
        } else if let Ok(f) = obj.downcast_exact::<PyFloat>() {
            Ok(Value::Float(f.extract()?))
        } else if let Ok(b) = obj.downcast_exact::<PyBool>() {
            Ok(Value::Bool(b.extract()?))
        } else if let Ok(v) = F32SparseVector::extract_bound(obj) {
            Ok(Value::SparseVector(SparseVector::F32 {
                indices: v.indices,
                values: v.values,
            }))
        } else if let Ok(_) = obj.downcast_exact::<PyNone>() {
            Ok(Value::Null())
        } else {
            Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to Value",
                obj.get_type().name()
            )))
        }
    }
}

impl<'py> IntoPyObject<'py> for Value {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        match self {
            Value::Null() => Ok(py.None().into_bound(py)),
            Value::String(s) => Ok(s.into_py_any(py)?.into_bound(py)),
            Value::Int(i) => Ok(i.into_py_any(py)?.into_bound(py)),
            Value::Float(f) => Ok(f.into_py_any(py)?.into_bound(py)),
            Value::Bool(b) => Ok(b.into_py_any(py)?.into_bound(py)),
            Value::Bytes(b) => Ok(b.into_py_any(py)?.into_bound(py)),
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
            Value::List(l) => {
                let list = PyList::empty(py);
                match &l.values {
                    Values::U8(values) => {
                        for value in values {
                            list.append(value.into_py_any(py)?)?;
                        }
                    }
                    Values::U32(values) => {
                        for value in values {
                            list.append(value.into_py_any(py)?)?;
                        }
                    }
                    Values::U64(values) => {
                        for value in values {
                            list.append(value.into_py_any(py)?)?;
                        }
                    }
                    Values::I8(values) => {
                        for value in values {
                            list.append(value.into_py_any(py)?)?;
                        }
                    }
                    Values::I32(values) => {
                        for value in values {
                            list.append(value.into_py_any(py)?)?;
                        }
                    }
                    Values::I64(values) => {
                        for value in values {
                            list.append(value.into_py_any(py)?)?;
                        }
                    }
                    Values::F32(values) => {
                        for value in values {
                            list.append(value.into_py_any(py)?)?;
                        }
                    }
                    Values::F64(values) => {
                        for value in values {
                            list.append(value.into_py_any(py)?)?;
                        }
                    }
                    Values::String(values) => {
                        for value in values {
                            list.append(value.into_py_any(py)?)?;
                        }
                    }
                }
                Ok(list.into_py_any(py)?.into_bound(py))
            }
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
            Some(topk_rs::proto::v1::data::value::Value::Binary(b)) => Value::Bytes(b.into()),
            Some(topk_rs::proto::v1::data::value::Value::Vector(v)) => match v.vector {
                Some(topk_rs::proto::v1::data::vector::Vector::Float(v)) =>
                {
                    #[allow(deprecated)]
                    Value::List(List {
                        values: Values::F32(v.values),
                    })
                }
                Some(topk_rs::proto::v1::data::vector::Vector::Byte(v)) =>
                {
                    #[allow(deprecated)]
                    Value::List(List {
                        values: Values::U8(v.values),
                    })
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
            Some(topk_rs::proto::v1::data::value::Value::List(l)) => Value::List(List {
                values: match l.values {
                    Some(topk_rs::proto::v1::data::list::Values::U8(values)) => {
                        Values::U8(values.values)
                    }
                    Some(topk_rs::proto::v1::data::list::Values::U32(values)) => {
                        Values::U32(values.values)
                    }
                    Some(topk_rs::proto::v1::data::list::Values::U64(values)) => {
                        Values::U64(values.values)
                    }
                    Some(topk_rs::proto::v1::data::list::Values::I8(values)) => {
                        // Transmuting to i8 from the `bytes` u8 representation in proto
                        Values::I8(values.into())
                    }
                    Some(topk_rs::proto::v1::data::list::Values::I32(values)) => {
                        Values::I32(values.values)
                    }
                    Some(topk_rs::proto::v1::data::list::Values::I64(values)) => {
                        Values::I64(values.values)
                    }
                    Some(topk_rs::proto::v1::data::list::Values::F32(values)) => {
                        Values::F32(values.values)
                    }
                    Some(topk_rs::proto::v1::data::list::Values::F64(values)) => {
                        Values::F64(values.values)
                    }
                    Some(topk_rs::proto::v1::data::list::Values::String(values)) => {
                        Values::String(values.values)
                    }
                    None => {
                        unreachable!("Invalid list proto: {:?}", l)
                    }
                },
            }),
            Some(topk_rs::proto::v1::data::value::Value::Struct(..)) => {
                todo!()
            }
            None => Value::Null(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeValue(pub(crate) Value);

impl<'py> IntoPyObject<'py> for NativeValue {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        self.0.into_pyobject(py)
    }
}

impl From<topk_rs::proto::v1::data::Value> for NativeValue {
    fn from(value: topk_rs::proto::v1::data::Value) -> Self {
        NativeValue(Value::from(value))
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
            Value::SparseVector(v) => match v {
                SparseVector::F32 { indices, values } => {
                    topk_rs::proto::v1::data::Value::f32_sparse_vector(indices, values)
                }
                SparseVector::U8 { indices, values } => {
                    topk_rs::proto::v1::data::Value::u8_sparse_vector(indices, values)
                }
            },
            Value::List(l) => match l.values {
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
