use pyo3::{prelude::*, pyclass, pymethods, IntoPyObject, IntoPyObjectExt};

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub(crate) values: Values,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Values {
    U8(Vec<u8>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    I8(Vec<i8>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    String(Vec<String>),
}

impl<'py> IntoPyObject<'py> for Values {
    type Target = pyo3::types::PyAny;
    type Output = pyo3::Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        match self {
            Values::U8(v) => Ok(v.into_py_any(py)?.into_bound(py)),
            Values::U32(v) => Ok(v.into_py_any(py)?.into_bound(py)),
            Values::U64(v) => Ok(v.into_py_any(py)?.into_bound(py)),
            Values::I8(v) => Ok(v.into_py_any(py)?.into_bound(py)),
            Values::I32(v) => Ok(v.into_py_any(py)?.into_bound(py)),
            Values::I64(v) => Ok(v.into_py_any(py)?.into_bound(py)),
            Values::F32(v) => Ok(v.into_py_any(py)?.into_bound(py)),
            Values::F64(v) => Ok(v.into_py_any(py)?.into_bound(py)),
            Values::String(v) => Ok(v.into_py_any(py)?.into_bound(py)),
        }
    }
}

#[pymethods]
impl List {
    fn __str__(&self) -> String {
        match &self.values {
            Values::U8(values) => format!("List(U8({:?}))", values),
            Values::U32(values) => format!("List(U32({:?}))", values),
            Values::U64(values) => format!("List(U64({:?}))", values),
            Values::I8(values) => format!("List(I8({:?}))", values),
            Values::I32(values) => format!("List(I32({:?}))", values),
            Values::I64(values) => format!("List(I64({:?}))", values),
            Values::F32(values) => format!("List(F32({:?}))", values),
            Values::F64(values) => format!("List(F64({:?}))", values),
            Values::String(values) => format!("List(String({:?}))", values),
        }
    }
}
