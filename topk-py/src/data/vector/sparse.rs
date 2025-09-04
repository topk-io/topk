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
        // First try to extract as a dict with indices and values keys
        if let Ok(dict) = obj.downcast::<PyDict>() {
            // Check if it's the new format with indices and values keys
            if dict.contains("indices")? && dict.contains("values")? {
                let indices_obj = dict.get_item("indices")?.ok_or_else(|| {
                    PyTypeError::new_err("Invalid sparse vector, 'indices' key not found")
                })?;
                let values_obj = dict.get_item("values")?.ok_or_else(|| {
                    PyTypeError::new_err("Invalid sparse vector, 'values' key not found")
                })?;

                let indices: Vec<u32> = indices_obj.extract()?;
                let values: Vec<f32> = values_obj.extract()?;
                
                if indices.len() != values.len() {
                    return Err(PyTypeError::new_err(
                        "Invalid sparse vector, indices and values must have the same length",
                    ));
                }
                
                // Validate that indices are sorted
                for i in 1..indices.len() {
                    if indices[i] <= indices[i - 1] {
                        return Err(PyTypeError::new_err(
                            "Invalid sparse vector, indices must be sorted in ascending order and unique",
                        ));
                    }
                }

                return Ok(F32SparseVector { indices, values });
            }
            
            // Otherwise, treat it as the old format {index: value}
            let mut indices = Vec::new();
            let mut values = Vec::new();

            for item in dict.items() {
                let (key, value) = item
                    .downcast_exact::<PyTuple>()
                    .map_err(|_| {
                        PyTypeError::new_err("Invalid sparse vector, must be `dict[int, float]` or `dict with 'indices' and 'values' keys`")
                    })?
                    .extract::<(u32, f32)>()
                    .map_err(|_| {
                        PyTypeError::new_err("Invalid sparse vector, must be `dict[int, float]` or `dict with 'indices' and 'values' keys`")
                    })?;

                indices.push(key);
                values.push(value);
            }

            return Ok(F32SparseVector { indices, values });
        }

        Err(PyTypeError::new_err("Invalid sparse vector, must be `dict[int, float]` or `dict with 'indices' and 'values' keys`"))
    }
}

impl<'py> FromPyObject<'py> for U8SparseVector {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        // First try to extract as a dict with indices and values keys
        if let Ok(dict) = obj.downcast::<PyDict>() {
            // Check if it's the new format with indices and values keys
            if dict.contains("indices")? && dict.contains("values")? {
                let indices_obj = dict.get_item("indices")?.ok_or_else(|| {
                    PyTypeError::new_err("Invalid sparse vector, 'indices' key not found")
                })?;
                let values_obj = dict.get_item("values")?.ok_or_else(|| {
                    PyTypeError::new_err("Invalid sparse vector, 'values' key not found")
                })?;

                let indices: Vec<u32> = indices_obj.extract()?;
                let values: Vec<u8> = values_obj.extract()?;
                
                if indices.len() != values.len() {
                    return Err(PyTypeError::new_err(
                        "Invalid sparse vector, indices and values must have the same length",
                    ));
                }
                
                // Validate that indices are sorted
                for i in 1..indices.len() {
                    if indices[i] <= indices[i - 1] {
                        return Err(PyTypeError::new_err(
                            "Invalid sparse vector, indices must be sorted in ascending order and unique",
                        ));
                    }
                }

                return Ok(U8SparseVector { indices, values });
            }
            
            // Otherwise, treat it as the old format {index: value}
            let mut indices = Vec::new();
            let mut values = Vec::new();

            for item in dict.items() {
                let (key, value) = item
                    .downcast_exact::<PyTuple>()
                    .map_err(|_| {
                        PyTypeError::new_err("Invalid sparse vector, must be `dict[int, int]` or `dict with 'indices' and 'values' keys`")
                    })?
                    .extract::<(u32, u8)>()
                    .map_err(|_| {
                        PyTypeError::new_err("Invalid sparse vector, must be `dict[int, int]` or `dict with 'indices' and 'values' keys`")
                    })?;

                indices.push(key);
                values.push(value);
            }

            return Ok(U8SparseVector { indices, values });
        }

        Err(PyTypeError::new_err("Invalid sparse vector, must be `dict[int, int]` or `dict with 'indices' and 'values' keys`"))
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
