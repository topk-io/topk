use half::f16;
use numpy::{
    dtype, PyArrayDescrMethods, PyArrayDyn, PyArrayMethods, PyUntypedArray, PyUntypedArrayMethods,
};
use pyo3::{pyclass, pymethods, Bound, PyErr, PyResult};

use crate::error::InvalidArgumentError;

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
    F8(Vec<float8::F8E4M3>),
    F16(Vec<half::f16>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    String(Vec<String>),
}

impl List {
    pub(crate) fn from_numpy_array(untyped: &Bound<'_, PyUntypedArray>) -> PyResult<Self> {
        let shape = untyped.shape();
        let ndim = shape.len();
        if ndim != 1 {
            return Err(InvalidArgumentError::new_err(format!(
                "Expected numpy array with ndim=1 for vector, got ndim={} array with shape {:?}",
                ndim, shape
            )));
        }

        let element_dtype = untyped.dtype();
        let py = untyped.py();

        if element_dtype.is_equiv_to(&dtype::<f32>(py)) {
            let array = untyped.cast::<PyArrayDyn<f32>>()?.readonly();
            return Ok(List {
                values: Values::F32(array.to_vec()?),
            });
        } else if element_dtype.is_equiv_to(&dtype::<f16>(py)) {
            let array = untyped.cast::<PyArrayDyn<f16>>()?.readonly();
            return Ok(List {
                values: Values::F16(array.to_vec()?),
            });
        } else if element_dtype.is_equiv_to(&dtype::<u8>(py)) {
            let array = untyped.cast::<PyArrayDyn<u8>>()?.readonly();
            return Ok(List {
                values: Values::U8(array.to_vec()?),
            });
        } else if element_dtype.is_equiv_to(&dtype::<i8>(py)) {
            let array = untyped.cast::<PyArrayDyn<i8>>()?.readonly();
            return Ok(List {
                values: Values::I8(array.to_vec()?),
            });
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "Unsupported numpy dtype: {}. Supported dtypes: float32, float16, uint8, int8. For f8, use f8_vector(values)",
            element_dtype
        )))
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
            Values::F8(values) => format!("List(F8({:?}))", values),
            Values::F16(values) => format!("List(F16({:?}))", values),
            Values::F32(values) => format!("List(F32({:?}))", values),
            Values::F64(values) => format!("List(F64({:?}))", values),
            Values::String(values) => format!("List(String({:?}))", values),
        }
    }
}
