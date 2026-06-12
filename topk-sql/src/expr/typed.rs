use topk_rs::proto::v1::data::Value;

use crate::Error;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ElemType {
    F16Vector,
    F32Vector,
    F64Vector,
    F8Vector,
    U8Vector,
    U32Vector,
    U64Vector,
    I8Vector,
    I32Vector,
    I64Vector,
}

impl ElemType {
    pub(crate) fn from_dense_type_name(type_name: &str) -> Option<Self> {
        Some(match type_name {
            "f16_vector" => Self::F16Vector,
            "f32_vector" => Self::F32Vector,
            "f64_vector" => Self::F64Vector,
            "f8_vector" => Self::F8Vector,
            "u8_vector" | "binary_vector" => Self::U8Vector,
            "u32_vector" => Self::U32Vector,
            "u64_vector" => Self::U64Vector,
            "i8_vector" => Self::I8Vector,
            "i32_vector" => Self::I32Vector,
            "i64_vector" => Self::I64Vector,
            _ => return None,
        })
    }

    pub(crate) fn from_sparse_type_name(type_name: &str) -> Option<Self> {
        Some(match type_name {
            "f16_sparse_vector" => Self::F16Vector,
            "f32_sparse_vector" => Self::F32Vector,
            "f8_sparse_vector" => Self::F8Vector,
            "u8_sparse_vector" => Self::U8Vector,
            "i8_sparse_vector" => Self::I8Vector,
            _ => return None,
        })
    }

    pub(crate) fn from_matrix_type_name(type_name: &str) -> Option<Self> {
        Some(match type_name {
            "f16_matrix" => Self::F16Vector,
            "f32_matrix" => Self::F32Vector,
            "f8_matrix" => Self::F8Vector,
            "u8_matrix" => Self::U8Vector,
            "i8_matrix" => Self::I8Vector,
            _ => return None,
        })
    }

    pub(crate) fn from_floats(self, values: Vec<f64>, sig: &str) -> Result<TypedValues, Error> {
        Ok(match self {
            Self::F16Vector => {
                TypedValues::F16(values.into_iter().map(half::f16::from_f64).collect())
            }
            Self::F32Vector => TypedValues::F32(values.into_iter().map(|f| f as f32).collect()),
            Self::F64Vector => TypedValues::F64(values),
            Self::F8Vector => {
                TypedValues::F8(values.into_iter().map(float8::F8E4M3::from_f64).collect())
            }
            Self::U8Vector => TypedValues::U8(coerce_floats(values, sig)?),
            Self::U32Vector => TypedValues::U32(coerce_floats(values, sig)?),
            Self::U64Vector => TypedValues::U64(coerce_floats(values, sig)?),
            Self::I8Vector => TypedValues::I8(coerce_floats(values, sig)?),
            Self::I32Vector => TypedValues::I32(coerce_floats(values, sig)?),
            Self::I64Vector => TypedValues::I64(coerce_floats(values, sig)?),
        })
    }

    pub(crate) fn from_i64s(self, values: Vec<i64>, sig: &str) -> Result<TypedValues, Error> {
        Ok(match self {
            Self::F16Vector => TypedValues::F16(
                values
                    .into_iter()
                    .map(|n| half::f16::from_f64(n as f64))
                    .collect(),
            ),
            Self::F32Vector => TypedValues::F32(values.into_iter().map(|n| n as f32).collect()),
            Self::F64Vector => TypedValues::F64(values.into_iter().map(|n| n as f64).collect()),
            Self::F8Vector => TypedValues::F8(
                values
                    .into_iter()
                    .map(|n| float8::F8E4M3::from_f64(n as f64))
                    .collect(),
            ),
            Self::U8Vector => TypedValues::U8(coerce_i64s(values, sig, "u8")?),
            Self::U32Vector => TypedValues::U32(coerce_i64s(values, sig, "u32")?),
            Self::U64Vector => TypedValues::U64(coerce_i64s(values, sig, "u64")?),
            Self::I8Vector => TypedValues::I8(coerce_i64s(values, sig, "i8")?),
            Self::I32Vector => TypedValues::I32(coerce_i64s(values, sig, "i32")?),
            Self::I64Vector => TypedValues::I64(coerce_i64s(values, sig, "i64")?),
        })
    }
}

pub(crate) enum TypedValues {
    F16(Vec<half::f16>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    F8(Vec<float8::F8E4M3>),
    U8(Vec<u8>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    I8(Vec<i8>),
    I32(Vec<i32>),
    I64(Vec<i64>),
}

impl TypedValues {
    pub(crate) fn into_list_value(self) -> Value {
        match self {
            Self::F16(v) => Value::list(v),
            Self::F32(v) => Value::list(v),
            Self::F64(v) => Value::list(v),
            Self::F8(v) => Value::list(v),
            Self::U8(v) => Value::list(v),
            Self::U32(v) => Value::list(v),
            Self::U64(v) => Value::list(v),
            Self::I8(v) => Value::list(v),
            Self::I32(v) => Value::list(v),
            Self::I64(v) => Value::list(v),
        }
    }

    pub(crate) fn into_sparse_value(self, indices: Vec<u32>, sig: &str) -> Result<Value, Error> {
        match self {
            Self::F16(v) => Ok(Value::f16_sparse_vector(indices, v)),
            Self::F32(v) => Ok(Value::f32_sparse_vector(indices, v)),
            Self::F8(v) => Ok(Value::f8_sparse_vector(indices, v)),
            Self::U8(v) => Ok(Value::u8_sparse_vector(indices, v)),
            Self::I8(v) => Ok(Value::i8_sparse_vector(indices, v)),
            _ => Err(Error::Invalid(format!(
                "{sig}: unsupported sparse vector element type"
            ))),
        }
    }

    pub(crate) fn into_matrix_value(self, cols: u32, sig: &str) -> Result<Value, Error> {
        match self {
            Self::F16(v) => Ok(Value::matrix(cols, v)),
            Self::F32(v) => Ok(Value::matrix(cols, v)),
            Self::F8(v) => Ok(Value::matrix(cols, v)),
            Self::U8(v) => Ok(Value::matrix(cols, v)),
            Self::I8(v) => Ok(Value::matrix(cols, v)),
            _ => Err(Error::Invalid(format!(
                "{sig}: unsupported matrix element type"
            ))),
        }
    }
}

pub(crate) fn coerce_i64s<T: TryFrom<i64>>(
    values: Vec<i64>,
    sig: &str,
    target: &str,
) -> Result<Vec<T>, Error> {
    values
        .into_iter()
        .map(|n| {
            T::try_from(n)
                .map_err(|_| Error::Invalid(format!("{sig}: element {n} out of {target} range")))
        })
        .collect()
}

fn coerce_floats<T: TryFrom<i64>>(values: Vec<f64>, sig: &str) -> Result<Vec<T>, Error> {
    values
        .into_iter()
        .map(|f| {
            if f.fract() != 0.0 || f.is_nan() || f.is_infinite() {
                return Err(Error::Invalid(format!(
                    "{sig}: element {f} is not an integer"
                )));
            }
            if f < i64::MIN as f64 || f > i64::MAX as f64 {
                return Err(Error::Invalid(format!("{sig}: element {f} out of range")));
            }
            let n = f as i64;
            T::try_from(n).map_err(|_| Error::Invalid(format!("{sig}: element {n} out of range")))
        })
        .collect()
}
