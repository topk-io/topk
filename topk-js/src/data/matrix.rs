use float8::F8E4M3;
use half::f16;
use napi_derive::napi;

/// Matrix element value type.
#[napi(string_enum = "lowercase", namespace = "data")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MatrixValueType {
    F32,
    F16,
    F8,
    U8,
    I8,
}

/// @internal
/// @hideconstructor
/// Instances of the `Matrix` class are used to represent matrices in TopK.
/// Usually created using data constructors such as [`matrix()`](#matrix).
#[derive(Debug, Clone, PartialEq)]
#[napi(namespace = "data")]
pub struct Matrix {
    pub(crate) num_cols: u32,
    pub(crate) values: MatrixValues,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatrixValues {
    F32(Vec<f32>),
    F16(Vec<f16>),
    F8(Vec<F8E4M3>),
    U8(Vec<u8>),
    I8(Vec<i8>),
}

#[napi(namespace = "data")]
impl Matrix {
    /// @ignore
    #[napi]
    pub fn to_string(&self) -> String {
        match &self.values {
            MatrixValues::F32(v) => format!("Matrix({}, F32({:?}))", self.num_cols, v),
            MatrixValues::F16(v) => {
                let vals: Vec<f32> = v.iter().map(|x| x.to_f32()).collect();
                format!("Matrix({}, F16({:?}))", self.num_cols, vals)
            }
            MatrixValues::F8(v) => {
                let vals: Vec<f32> = v.iter().map(|x| x.to_f32()).collect();
                format!("Matrix({}, F8({:?}))", self.num_cols, vals)
            }
            MatrixValues::U8(v) => format!("Matrix({}, U8({:?}))", self.num_cols, v),
            MatrixValues::I8(v) => format!("Matrix({}, I8({:?}))", self.num_cols, v),
        }
    }
}

impl Matrix {
    pub(crate) fn from_list_of_lists(
        values: Vec<Vec<f64>>,
        value_type: Option<MatrixValueType>,
    ) -> napi::Result<Self> {
        if values.is_empty() {
            return Err(napi::Error::from_reason(
                "Cannot create matrix from empty list",
            ));
        }

        let num_cols = values[0].len();
        let num_cols: u32 = num_cols
            .try_into()
            .map_err(|_| napi::Error::from_reason("num_cols is too large"))?;

        if num_cols == 0 {
            return Err(napi::Error::from_reason(
                "Cannot create matrix from empty list",
            ));
        }

        // Validate that all rows have the same length
        for (idx, row) in values.iter().enumerate() {
            if row.len() != num_cols as usize {
                return Err(napi::Error::from_reason(format!(
                    "All rows must have the same length. Row {} has length {}, but expected {}",
                    idx,
                    row.len(),
                    num_cols
                )));
            }
        }

        // Flatten all values
        let flattened: Vec<f64> = values.iter().flat_map(|row| row.iter().copied()).collect();

        if flattened.is_empty() {
            return Err(napi::Error::from_reason(
                "Cannot create matrix from empty list",
            ));
        }

        let values = match value_type {
            Some(MatrixValueType::F32) | None => {
                MatrixValues::F32(flattened.into_iter().map(|v| v as f32).collect())
            }
            Some(MatrixValueType::F16) => MatrixValues::F16(
                flattened
                    .into_iter()
                    .map(|v| f16::from_f32(v as f32))
                    .collect(),
            ),
            Some(MatrixValueType::F8) => MatrixValues::F8(
                flattened
                    .into_iter()
                    .map(|v| F8E4M3::from_f32(v as f32))
                    .collect(),
            ),
            Some(MatrixValueType::U8) => {
                let mut out = Vec::with_capacity(flattened.len());
                for v in flattened {
                    if v.fract() != 0.0 {
                        return Err(napi::Error::from_reason(
                            "'float' object cannot be interpreted as an integer",
                        ));
                    }
                    let n = v as i64;
                    if n < 0 || n > 255 {
                        return Err(napi::Error::from_reason("u8 value out of range"));
                    }
                    out.push(n as u8);
                }
                MatrixValues::U8(out)
            }
            Some(MatrixValueType::I8) => {
                let mut out = Vec::with_capacity(flattened.len());
                for v in flattened {
                    if v.fract() != 0.0 {
                        return Err(napi::Error::from_reason(
                            "'float' object cannot be interpreted as an integer",
                        ));
                    }
                    let n = v as i64;
                    if n < -128 || n > 127 {
                        return Err(napi::Error::from_reason("i8 value out of range"));
                    }
                    out.push(n as i8);
                }
                MatrixValues::I8(out)
            }
        };

        Ok(Self { num_cols, values })
    }
}

impl From<Matrix> for topk_rs::proto::v1::data::Value {
    fn from(matrix: Matrix) -> Self {
        match matrix.values {
            MatrixValues::F32(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::F16(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::F8(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::U8(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::I8(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
        }
    }
}
