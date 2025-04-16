use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyBool, PyFloat, PyInt, PyList, PyNone, PyString},
    IntoPyObjectExt,
};

#[derive(Debug, PartialEq, Clone)]
pub enum ValueUnion {
    Null(),
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    FloatVector(Vec<f32>),
    ByteVector(Vec<u8>),
}

impl<'py> FromPyObject<'py> for ValueUnion {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let obj = ob.as_ref();

        if let Ok(s) = obj.downcast::<PyString>() {
            Ok(ValueUnion::String(s.extract()?))
        } else if let Ok(i) = obj.downcast::<PyInt>() {
            Ok(ValueUnion::Int(i.extract()?))
        } else if let Ok(f) = obj.downcast::<PyFloat>() {
            Ok(ValueUnion::Float(f.extract()?))
        } else if let Ok(b) = obj.downcast::<PyBool>() {
            Ok(ValueUnion::Bool(b.extract()?))
        } else if let Ok(v) = obj.downcast::<PyList>() {
            // Try converting to vector from starting with most restrictive type first.
            if let Ok(values) = v.extract::<Vec<u8>>() {
                Ok(ValueUnion::ByteVector(values))
            } else if let Ok(values) = v.extract::<Vec<f32>>() {
                Ok(ValueUnion::FloatVector(values))
            } else {
                Err(PyTypeError::new_err(format!(
                    "Can't convert from {:?} to Value",
                    obj.get_type().name()
                )))
            }
        } else if let Ok(_) = obj.downcast::<PyNone>() {
            Ok(ValueUnion::Null())
        } else {
            Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to Value",
                obj.get_type().name()
            )))
        }
    }
}

impl<'py> IntoPyObject<'py> for ValueUnion {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        match self {
            ValueUnion::Null() => Ok(py.None().into_bound(py)),
            ValueUnion::String(s) => Ok(s.into_py_any(py)?.into_bound(py)),
            ValueUnion::Int(i) => Ok(i.into_py_any(py)?.into_bound(py)),
            ValueUnion::Float(f) => Ok(f.into_py_any(py)?.into_bound(py)),
            ValueUnion::Bool(b) => Ok(b.into_py_any(py)?.into_bound(py)),
            ValueUnion::FloatVector(v) => Ok(v.into_py_any(py)?.into_bound(py)),
            ValueUnion::ByteVector(v) => Ok(v.into_py_any(py)?.into_bound(py)),
        }
    }
}

impl From<topk_protos::v1::data::Value> for ValueUnion {
    fn from(value: topk_protos::v1::data::Value) -> Self {
        match value.value {
            Some(topk_protos::v1::data::value::Value::String(s)) => ValueUnion::String(s),
            Some(topk_protos::v1::data::value::Value::U32(i)) => ValueUnion::Int(i as i64),
            Some(topk_protos::v1::data::value::Value::U64(i)) => ValueUnion::Int(i as i64),
            Some(topk_protos::v1::data::value::Value::I64(i)) => ValueUnion::Int(i as i64),
            Some(topk_protos::v1::data::value::Value::I32(i)) => ValueUnion::Int(i as i64),
            Some(topk_protos::v1::data::value::Value::F32(f)) => ValueUnion::Float(f as f64),
            Some(topk_protos::v1::data::value::Value::F64(f)) => ValueUnion::Float(f),
            Some(topk_protos::v1::data::value::Value::Bool(b)) => ValueUnion::Bool(b),
            Some(topk_protos::v1::data::value::Value::Null(_)) => ValueUnion::Null(),
            Some(topk_protos::v1::data::value::Value::Binary(_)) => todo!(),
            Some(topk_protos::v1::data::value::Value::Vector(v)) => match v.vector {
                Some(topk_protos::v1::data::vector::Vector::Float(v)) => {
                    ValueUnion::FloatVector(v.values)
                }
                Some(topk_protos::v1::data::vector::Vector::Byte(v)) => {
                    ValueUnion::ByteVector(v.values)
                }
                _ => todo!(),
            },
            None => ValueUnion::Null(),
        }
    }
}

impl From<ValueUnion> for topk_protos::v1::data::Value {
    fn from(value: ValueUnion) -> Self {
        Self {
            value: Some(match value {
                ValueUnion::Bool(b) => topk_protos::v1::data::value::Value::Bool(b),
                ValueUnion::Int(i) => topk_protos::v1::data::value::Value::I64(i),
                ValueUnion::Float(f) => topk_protos::v1::data::value::Value::F64(f),
                ValueUnion::String(s) => topk_protos::v1::data::value::Value::String(s),
                ValueUnion::Null() => {
                    topk_protos::v1::data::value::Value::Null(topk_protos::v1::data::Null {})
                }
                ValueUnion::FloatVector(v) => topk_protos::v1::data::value::Value::Vector(
                    topk_protos::v1::data::Vector::float(v),
                ),
                ValueUnion::ByteVector(v) => topk_protos::v1::data::value::Value::Vector(
                    topk_protos::v1::data::Vector::byte(v),
                ),
            }),
        }
    }
}
