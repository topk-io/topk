use pyo3::{exceptions::PyTypeError, prelude::*};

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub enum Vector {
    F32(Vec<f32>),
    U8(Vec<u8>),
}

#[pymethods]
impl Vector {
    fn __str__(&self) -> String {
        match self {
            Vector::F32(values) => format!("Vector(F32({:?}))", values),
            Vector::U8(values) => format!("Vector(U8({:?}))", values),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct F32Vector {
    pub(crate) values: Vec<f32>,
}

impl From<F32Vector> for Vector {
    fn from(vector: F32Vector) -> Self {
        Vector::F32(vector.values)
    }
}

impl<'py> FromPyObject<'py> for F32Vector {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(values) = Vec::<f32>::extract_bound(obj) {
            return Ok(F32Vector { values });
        }

        Err(PyTypeError::new_err(
            "Invalid vector value, must be `list[float]`",
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct U8Vector {
    pub(crate) values: Vec<u8>,
}

impl From<U8Vector> for Vector {
    fn from(vector: U8Vector) -> Self {
        Vector::U8(vector.values)
    }
}

impl<'py> FromPyObject<'py> for U8Vector {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(values) = Vec::<u8>::extract_bound(obj) {
            return Ok(U8Vector { values });
        }

        Err(PyTypeError::new_err(
            "Invalid vector value, must be `list[int]`",
        ))
    }
}

impl From<Vector> for topk_rs::proto::v1::data::Vector {
    fn from(vector: Vector) -> Self {
        match vector {
            Vector::F32(values) => topk_rs::proto::v1::data::Vector::f32(values),
            Vector::U8(values) => topk_rs::proto::v1::data::Vector::u8(values),
        }
    }
}
