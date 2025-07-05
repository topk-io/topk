use crate::data::value::Value;
use crate::data::vector::{
    F32SparseVector, F32Vector, SparseVector, U8SparseVector, U8Vector, Vector,
};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};

pub mod collection;
pub mod document;
pub mod scalar;
pub mod value;
pub mod vector;

////////////////////////////////////////////////////////////
/// Query
///
/// This module contains the query definition for the TopK SDK.
////////////////////////////////////////////////////////////

#[pymodule]
#[pyo3(name = "data")]
pub fn pymodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // (Dense) Vectors
    m.add_wrapped(wrap_pyfunction!(f32_vector))?;
    m.add_wrapped(wrap_pyfunction!(u8_vector))?;
    m.add_wrapped(wrap_pyfunction!(binary_vector))?;
    // Sparse vectors
    m.add_wrapped(wrap_pyfunction!(f32_sparse_vector))?;
    m.add_wrapped(wrap_pyfunction!(u8_sparse_vector))?;
    // Bytes
    m.add_wrapped(wrap_pyfunction!(bytes))?;

    Ok(())
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn f32_vector(vector: F32Vector) -> Vector {
    vector.into()
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn u8_vector(vector: U8Vector) -> Vector {
    vector.into()
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn binary_vector(vector: U8Vector) -> Vector {
    vector.into()
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn f32_sparse_vector(vector: F32SparseVector) -> SparseVector {
    vector.into()
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn u8_sparse_vector(vector: U8SparseVector) -> SparseVector {
    vector.into()
}

#[pyfunction]
#[pyo3(signature = (data))]
pub fn bytes(data: &Bound<'_, PyAny>) -> PyResult<Value> {
    if let Ok(py_bytes) = data.downcast::<PyBytes>() {
        let bytes_vec = py_bytes.as_bytes().to_vec();
        Ok(Value::Bytes(bytes_vec))
    } else if let Ok(py_list) = data.downcast::<PyList>() {
        let bytes_vec: Vec<u8> = py_list.extract().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Expected list[int] with values in range [0, 255]",
            ))
        })?;
        Ok(Value::Bytes(bytes_vec))
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected bytes or list[int] for bytes() function",
        ))
    }
}
