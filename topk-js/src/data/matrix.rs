use float8::F8E4M3;
use napi_derive::napi;

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
    F16(Vec<half::f16>),
    F8(Vec<F8E4M3>),
    U8(Vec<u8>),
    I8(Vec<i8>),
}

#[napi(string_enum = "lowercase", namespace = "data")]
#[derive(Clone, Debug)]
pub enum MatrixValueType {
    F32,
    F16,
    F8,
    U8,
    I8,
}

/// @internal
#[napi(namespace = "data")]
impl Matrix {
    /// @ignore
    #[napi]
    pub fn to_string(&self) -> String {
        format!("Matrix({}, {:?})", self.num_cols, self.values)
    }
}

impl From<Matrix> for topk_rs::proto::v1::data::Matrix {
    fn from(matrix: Matrix) -> Self {
        // Use the Value::matrix() helper which internally uses IntoMatrixValues
        match matrix.values {
            MatrixValues::F32(v) => topk_rs::proto::v1::data::Matrix::new(matrix.num_cols, v),
            MatrixValues::U8(v) => topk_rs::proto::v1::data::Matrix::new(matrix.num_cols, v),
            MatrixValues::I8(v) => topk_rs::proto::v1::data::Matrix::new(matrix.num_cols, v),
            MatrixValues::F16(v) => topk_rs::proto::v1::data::Matrix::new(matrix.num_cols, v),
            MatrixValues::F8(v) => topk_rs::proto::v1::data::Matrix::new(matrix.num_cols, v),
        }
    }
}

impl From<Matrix> for topk_rs::proto::v1::data::Value {
    fn from(matrix: Matrix) -> Self {
        // Use the Value::matrix() helper which internally uses IntoMatrixValues
        match matrix.values {
            MatrixValues::F32(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::U8(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::I8(v) => topk_rs::proto::v1::data::Value::matrix(matrix.num_cols, v),
            MatrixValues::F16(_) | MatrixValues::F8(_) => {
                unreachable!("F16/F8 matrix support not yet fully implemented")
            }
        }
    }
}
