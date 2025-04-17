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
    m.add_wrapped(wrap_pyfunction!(f32_vector))?;
    m.add_wrapped(wrap_pyfunction!(u8_vector))?;
    m.add_wrapped(wrap_pyfunction!(binary_vector))?;

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
