use crate::data::vector::{F32SparseVector, F32Vector, U8SparseVector, U8Vector};
use pyo3::prelude::*;

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
pub fn f32_vector(values: F32Vector) -> PyResult<value::Value> {
    Ok(value::Value::Vector(values.into()))
}

#[pyfunction]
#[pyo3(signature = (values))]
pub fn u8_vector(values: U8Vector) -> PyResult<value::Value> {
    Ok(value::Value::Vector(values.into()))
}

#[pyfunction]
#[pyo3(signature = (values))]
pub fn binary_vector(values: U8Vector) -> PyResult<value::Value> {
    Ok(value::Value::Vector(values.into()))
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn f32_sparse_vector(vector: F32SparseVector) -> PyResult<value::Value> {
    Ok(value::Value::SparseVector(vector.into()))
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn u8_sparse_vector(vector: U8SparseVector) -> PyResult<value::Value> {
    Ok(value::Value::SparseVector(vector.into()))
}
