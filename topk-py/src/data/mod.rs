use pyo3::{prelude::*, types::PyDict};

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

    Ok(())
}

#[pyfunction]
#[pyo3(signature = (values))]
pub fn f32_vector(values: Vec<f32>) -> PyResult<value::Value> {
    Ok(value::Value::Vector(vector::Vector::F32(values)))
}

#[pyfunction]
#[pyo3(signature = (values))]
pub fn u8_vector(values: Vec<u8>) -> PyResult<value::Value> {
    Ok(value::Value::Vector(vector::Vector::U8(values)))
}

#[pyfunction]
#[pyo3(signature = (values))]
pub fn binary_vector(values: Vec<u8>) -> PyResult<value::Value> {
    Ok(value::Value::Vector(vector::Vector::U8(values)))
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn f32_sparse_vector(vector: &Bound<'_, PyDict>) -> PyResult<value::Value> {
    Ok(value::Value::SparseVector(vector::SparseVector::F32 {
        indices: vector.keys().extract::<Vec<u32>>()?,
        values: vector.values().extract::<Vec<f32>>()?,
    }))
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn u8_sparse_vector(vector: &Bound<'_, PyDict>) -> PyResult<value::Value> {
    Ok(value::Value::SparseVector(vector::SparseVector::U8 {
        indices: vector.keys().extract::<Vec<u32>>()?,
        values: vector.values().extract::<Vec<u8>>()?,
    }))
}
