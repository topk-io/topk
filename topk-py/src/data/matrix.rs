use float8::F8E4M3;
use half::f16;
use numpy::{
    dtype, PyArrayDescrMethods, PyArrayDyn, PyArrayMethods, PyUntypedArray, PyUntypedArrayMethods,
};
use pyo3::{
    pyclass, pymethods,
    types::{PyAnyMethods, PyList, PyListMethods},
    Borrowed, Bound, PyAny, PyErr, PyResult,
};

use crate::error::InvalidArgumentError;

/// @internal
/// @hideconstructor
/// Instances of the `Matrix` class are used to represent matrices in TopK.
/// Usually created using data constructors such as [`matrix()`](#matrix).
#[pyclass(frozen)]
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub(crate) num_cols: u32,
    pub(crate) values: MatrixValues,
}

impl Matrix {
    pub(crate) fn from_numpy_array(untyped: &Bound<'_, PyUntypedArray>) -> PyResult<Self> {
        if true {
            use numpy::{
                dtype, PyArrayDescrMethods, PyArrayDyn, PyArrayMethods, PyUntypedArray,
                PyUntypedArrayMethods,
            };
        }

        let shape = untyped.shape();
        let ndim = shape.len();
        let num_cols = match ndim {
            1 => Ok(shape[0] as u32), // 1D: single row, num_cols = length
            2 => Ok(shape[1] as u32), // 2D: num_cols is second dimension
            _ => Err(InvalidArgumentError::new_err(format!(
                "Expected numpy array with ndim=1 or ndim=2, got ndim={} array with shape {:?}",
                ndim, shape
            ))),
        }?;

        // If numpy array is empty (has any dimension of size 0) return error
        if shape.iter().any(|&dim| dim == 0) {
            return Err(InvalidArgumentError::new_err(
                "Cannot create matrix from empty list",
            ));
        }

        // Detect the value type from the numpy array
        let element_dtype = untyped.dtype();
        let py = untyped.py();

        if element_dtype.is_equiv_to(&dtype::<f32>(py)) {
            let array = untyped.cast::<PyArrayDyn<f32>>()?.readonly();
            return Ok(Matrix {
                num_cols,
                values: MatrixValues::F32(array.to_vec()?),
            });
        } else if element_dtype.is_equiv_to(&dtype::<f16>(py)) {
            let array = untyped.cast::<PyArrayDyn<f16>>()?.readonly();
            return Ok(Matrix {
                num_cols,
                values: MatrixValues::F16(array.to_vec()?),
            });
        } else if element_dtype.is_equiv_to(&dtype::<u8>(py)) {
            let array = untyped.cast::<PyArrayDyn<u8>>()?.readonly();
            return Ok(Matrix {
                num_cols,
                values: MatrixValues::U8(array.to_vec()?),
            });
        } else if element_dtype.is_equiv_to(&dtype::<i8>(py)) {
            let array = untyped.cast::<PyArrayDyn<i8>>()?.readonly();
            return Ok(Matrix {
                num_cols,
                values: MatrixValues::I8(array.to_vec()?),
            });
        }

        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "Unsupported numpy dtype: {}. Supported dtypes: float32, float16, uint8, int8. For f8, use matrix(values, value_type='f8')",
            element_dtype
        )));
    }

    pub(crate) fn from_list_of_lists(
        obj: &Borrowed<'_, '_, PyAny>,
        value_type: Option<&str>,
    ) -> PyResult<Self> {
        let list = obj.cast::<PyList>()?;
        if list.is_empty() {
            return Err(InvalidArgumentError::new_err(
                "Cannot create matrix from empty list",
            ));
        }

        let num_cols = list.get_item(0)?.cast::<PyList>()?.len();

        if num_cols == 0 {
            return Err(InvalidArgumentError::new_err(
                "Cannot create matrix from empty list",
            ));
        }

        // Helper function to flatten list of lists and validate row lengths
        fn flatten_and_validate<E, T, F>(
            list: &Bound<'_, PyList>,
            num_cols: usize,
            extract_and_convert: F,
        ) -> PyResult<Vec<T>>
        where
            E: for<'a> pyo3::FromPyObject<'a, 'a, Error = PyErr>,
            F: Fn(E) -> T,
        {
            let mut flattened = Vec::new();
            for (idx, item) in list.iter().enumerate() {
                let row = item.cast::<PyList>()?;
                // Validate row length
                if row.len() != num_cols {
                    return Err(InvalidArgumentError::new_err(format!(
                        "All rows must have the same length. Row {} has length {}, but expected {}",
                        idx,
                        row.len(),
                        num_cols
                    )));
                }
                for val in row.iter() {
                    let extracted: E = val.extract::<E>()?;
                    flattened.push(extract_and_convert(extracted));
                }
            }

            if flattened.is_empty() {
                return Err(InvalidArgumentError::new_err(
                    "Cannot create matrix from empty list",
                ));
            }

            Ok(flattened)
        }

        // Convert based on provided value_type
        let values = match value_type {
            Some(vt) => match vt {
                "f32" => {
                    MatrixValues::F32(flatten_and_validate::<f32, f32, _>(&list, num_cols, |v| v)?)
                }
                "f16" => MatrixValues::F16(flatten_and_validate::<f32, half::f16, _>(
                    &list,
                    num_cols,
                    half::f16::from_f32,
                )?),
                "f8" => MatrixValues::F8(flatten_and_validate::<f32, F8E4M3, _>(
                    &list,
                    num_cols,
                    F8E4M3::from_f32,
                )?),
                "u8" => {
                    MatrixValues::U8(flatten_and_validate::<u8, u8, _>(&list, num_cols, |v| v)?)
                }
                "i8" => {
                    MatrixValues::I8(flatten_and_validate::<i8, i8, _>(&list, num_cols, |v| v)?)
                }
                _ => {
                    return Err(InvalidArgumentError::new_err(format!(
                        "Unsupported value_type: {}. Supported types: f8, f16, f32, u8, i8",
                        vt
                    )));
                }
            },
            None => {
                // Default to f32
                MatrixValues::F32(flatten_and_validate::<f32, f32, _>(&list, num_cols, |v| v)?)
            }
        };

        Ok(Self {
            num_cols: num_cols as u32,
            values,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatrixValues {
    F32(Vec<f32>),
    F16(Vec<half::f16>),
    F8(Vec<F8E4M3>),
    U8(Vec<u8>),
    I8(Vec<i8>),
}

#[pymethods]
impl Matrix {
    fn __str__(&self) -> String {
        format!("Matrix({}, {:?})", self.num_cols, self.values)
    }
}

impl From<Matrix> for topk_rs::proto::v1::data::Value {
    fn from(matrix: Matrix) -> Self {
        match matrix.values {
            MatrixValues::F32(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::U8(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::I8(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::F16(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::F8(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
        }
    }
}
