use crate::data::list::List;
use crate::data::value::Value;
use crate::data::vector::{F32SparseVector, SparseVector, U8SparseVector};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};

pub mod collection;
pub mod list;
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
    // List
    m.add_wrapped(wrap_pyfunction!(u32_list))?;
    m.add_wrapped(wrap_pyfunction!(i32_list))?;
    m.add_wrapped(wrap_pyfunction!(i64_list))?;
    m.add_wrapped(wrap_pyfunction!(f64_list))?;

    Ok(())
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn f32_vector(vector: Vec<f32>) -> List {
    List {
        values: list::Values::F32(vector),
    }
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn u8_vector(vector: Vec<u8>) -> List {
    List {
        values: list::Values::U8(vector),
    }
}

#[pyfunction]
#[pyo3(signature = (vector))]
pub fn binary_vector(vector: Vec<u8>) -> List {
    List {
        values: list::Values::U8(vector),
    }
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

#[pyfunction]
pub fn u32_list(data: &Bound<'_, PyAny>) -> PyResult<Value> {
    if let Ok(s) = data.extract::<Vec<u32>>() {
        return Ok(Value::List(list::List {
            values: list::Values::U32(s),
        }));
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[int] for u32_list() function",
        ))
    }
}

#[pyfunction]
pub fn i32_list(data: &Bound<'_, PyAny>) -> PyResult<Value> {
    if let Ok(s) = data.extract::<Vec<i32>>() {
        return Ok(Value::List(list::List {
            values: list::Values::I32(s),
        }));
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[int] for i32_list() function",
        ))
    }
}

#[pyfunction]
pub fn i64_list(data: &Bound<'_, PyAny>) -> PyResult<Value> {
    if let Ok(s) = data.extract::<Vec<i64>>() {
        return Ok(Value::List(list::List {
            values: list::Values::I64(s),
        }));
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[int] for i64_list() function",
        ))
    }
}

#[pyfunction]
pub fn f64_list(data: &Bound<'_, PyAny>) -> PyResult<Value> {
    if let Ok(s) = data.extract::<Vec<f64>>() {
        return Ok(Value::List(list::List {
            values: list::Values::F64(s),
        }));
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[float] for f64_list() function",
        ))
    }
}
