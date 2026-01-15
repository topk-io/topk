use crate::data::list::List;
use crate::data::matrix::Matrix;
use crate::data::value::Value;
use crate::data::vector::{F32SparseVector, SparseVector, U8SparseVector};
use numpy::PyUntypedArray;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};

pub mod collection;
pub mod list;
pub mod matrix;
pub mod value;
pub mod vector;

////////////////////////////////////////////////////////////
/// This module contains the data constructors for the TopK SDK.
////////////////////////////////////////////////////////////

#[pymodule]
#[pyo3(name = "data")]
pub fn pymodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // classes
    m.add_class::<List>()?;
    m.add_class::<SparseVector>()?;
    m.add_class::<Matrix>()?;

    // (Dense) Vectors
    m.add_wrapped(wrap_pyfunction!(f32_vector))?;
    m.add_wrapped(wrap_pyfunction!(u8_vector))?;
    m.add_wrapped(wrap_pyfunction!(i8_vector))?;
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
    m.add_wrapped(wrap_pyfunction!(f32_list))?;
    m.add_wrapped(wrap_pyfunction!(f64_list))?;
    m.add_wrapped(wrap_pyfunction!(string_list))?;
    // Matrix
    m.add_wrapped(wrap_pyfunction!(matrix_))?;

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
pub fn i8_vector(vector: Vec<i8>) -> List {
    List {
        values: list::Values::I8(vector),
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
    if let Ok(py_bytes) = data.cast::<PyBytes>() {
        let bytes_vec = py_bytes.as_bytes().to_vec();
        Ok(Value::Bytes(bytes_vec))
    } else if let Ok(py_list) = data.cast::<PyList>() {
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
pub fn u32_list(data: &Bound<'_, PyAny>) -> PyResult<List> {
    if let Ok(s) = data.extract::<Vec<u32>>() {
        return Ok(List {
            values: list::Values::U32(s),
        });
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[int] for u32_list() function",
        ))
    }
}

#[pyfunction]
pub fn i32_list(data: &Bound<'_, PyAny>) -> PyResult<List> {
    if let Ok(s) = data.extract::<Vec<i32>>() {
        return Ok(List {
            values: list::Values::I32(s),
        });
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[int] for i32_list() function",
        ))
    }
}

#[pyfunction]
pub fn i64_list(data: &Bound<'_, PyAny>) -> PyResult<List> {
    if let Ok(s) = data.extract::<Vec<i64>>() {
        return Ok(List {
            values: list::Values::I64(s),
        });
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[int] for i64_list() function",
        ))
    }
}

#[pyfunction]
pub fn f32_list(data: &Bound<'_, PyAny>) -> PyResult<List> {
    if let Ok(s) = data.extract::<Vec<f32>>() {
        return Ok(List {
            values: list::Values::F32(s),
        });
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[float] for f32_list() function",
        ))
    }
}

#[pyfunction]
pub fn f64_list(data: &Bound<'_, PyAny>) -> PyResult<List> {
    if let Ok(s) = data.extract::<Vec<f64>>() {
        return Ok(List {
            values: list::Values::F64(s),
        });
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[float] for f64_list() function",
        ))
    }
}

#[pyfunction]
pub fn string_list(data: &Bound<'_, PyAny>) -> PyResult<List> {
    if let Ok(s) = data.extract::<Vec<String>>() {
        return Ok(List {
            values: list::Values::String(s),
        });
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected list[str] for string_list() function",
        ))
    }
}

#[pyfunction]
#[pyo3(name = "matrix", signature = (values, value_type = None))]
pub fn matrix_(values: &Bound<'_, PyAny>, value_type: Option<String>) -> PyResult<Matrix> {
    // Check if it's a numpy array
    if let Ok(untyped) = values.cast::<PyUntypedArray>() {
        Ok(Matrix::from_numpy_array(untyped)?)
    } else {
        // Not a numpy array - parse matrix from python list of lists
        Ok(Matrix::from_list_of_lists(
            &values.as_borrowed(),
            value_type.as_deref(),
        )?)
    }
}
