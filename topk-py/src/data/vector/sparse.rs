use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyDict, PyTuple},
};

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub enum SparseVector {
    F32 { indices: Vec<u32>, values: Vec<f32> },
    U8 { indices: Vec<u32>, values: Vec<u8> },
}

#[derive(Debug, PartialEq, Clone)]
pub struct F32SparseVector {
    pub(crate) indices: Vec<u32>,
    pub(crate) values: Vec<f32>,
}

#[pymethods]
impl SparseVector {
    fn __str__(&self) -> String {
        match self {
            SparseVector::F32 { indices, values } => {
                format!("SparseVector(F32({:?}, {:?}))", indices, values)
            }
            SparseVector::U8 { indices, values } => {
                format!("SparseVector(U8({:?}, {:?}))", indices, values)
            }
        }
    }
}

impl From<F32SparseVector> for SparseVector {
    fn from(sparse: F32SparseVector) -> Self {
        SparseVector::F32 {
            indices: sparse.indices,
            values: sparse.values,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct U8SparseVector {
    pub(crate) indices: Vec<u32>,
    pub(crate) values: Vec<u8>,
}

impl From<U8SparseVector> for SparseVector {
    fn from(sparse: U8SparseVector) -> Self {
        SparseVector::U8 {
            indices: sparse.indices,
            values: sparse.values,
        }
    }
}

impl<'py> FromPyObject<'py> for F32SparseVector {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        let dict = obj.downcast_exact::<PyDict>().map_err(|_| {
            PyTypeError::new_err("Invalid sparse vector, must be `dict[int, float]`")
        })?;
        let mut indices = Vec::new();
        let mut values = Vec::new();

        for item in dict.items() {
            let (key, value) = item
                .downcast_exact::<PyTuple>()
                .map_err(|_| {
                    PyTypeError::new_err("Invalid sparse vector, must be `dict[int, float]`")
                })?
                .extract::<(u32, f32)>()
                .map_err(|_| {
                    PyTypeError::new_err("Invalid sparse vector, must be `dict[int, float]`")
                })?;

            indices.push(key);
            values.push(value);
        }

        Ok(F32SparseVector { indices, values })
    }
}

impl<'py> FromPyObject<'py> for U8SparseVector {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        let dict = obj
            .downcast_exact::<PyDict>()
            .map_err(|_| PyTypeError::new_err("Invalid sparse vector, must be `dict[int, int]`"))?;
        let mut indices = Vec::new();
        let mut values = Vec::new();

        for item in dict.items() {
            let (key, value) = item
                .downcast_exact::<PyTuple>()
                .map_err(|_| {
                    PyTypeError::new_err("Invalid sparse vector, must be `dict[int, int]`")
                })?
                .extract::<(u32, u8)>()
                .map_err(|_| {
                    PyTypeError::new_err("Invalid sparse vector, must be `dict[int, int]`")
                })?;

            indices.push(key);
            values.push(value);
        }

        Ok(U8SparseVector { indices, values })
    }
}

impl From<SparseVector> for topk_rs::proto::v1::data::SparseVector {
    fn from(sparse: SparseVector) -> Self {
        match sparse {
            SparseVector::F32 { indices, values } => {
                topk_rs::proto::v1::data::SparseVector::f32(indices, values)
            }
            SparseVector::U8 { indices, values } => {
                topk_rs::proto::v1::data::SparseVector::u8(indices, values)
            }
        }
    }
}
