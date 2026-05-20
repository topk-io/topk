use super::vector::SparseVector;
use crate::data::{
    list::{List, Values},
    matrix::{Matrix, MatrixValueType, MatrixValues},
    r#struct::Struct,
    vector::{SparseVectorData, SparseVectorUnion},
};
use napi::{bindgen_prelude::*, sys::napi_is_buffer};
use std::{collections::HashMap, ffi::CString, ptr};

/// Validates and deserializes a plain JS object into struct fields.
/// Rejects arrays, Date, Map, Set, and class instances at the JS boundary.
pub(crate) struct PlainJsObject {
    pub(crate) fields: HashMap<String, Value>,
}

impl FromNapiValue for PlainJsObject {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let mut is_array = false;
        check_status!(napi::sys::napi_is_array(env, napi_val, &mut is_array))?;
        if is_array {
            return Err(napi::Error::from_reason(
                "struct() expects a plain object, not an array".to_string(),
            ));
        }

        let object = Object::from_napi_value(env, napi_val)?;

        // Reject non-plain objects (Date, Map, Set, class instances).
        if let Ok(ctor) = object.get_named_property::<Unknown>("constructor") {
            let ctor_napi = Unknown::to_napi_value(env, ctor)?;
            let mut ctor_type: i32 = 0;
            check_status!(napi::sys::napi_typeof(env, ctor_napi, &mut ctor_type))?;
            if ctor_type == napi::sys::ValueType::napi_function {
                if let Ok(ctor_obj) = Object::from_napi_value(env, ctor_napi) {
                    if let Ok(name) = ctor_obj.get_named_property::<String>("name") {
                        if name != "Object" {
                            return Err(napi::Error::from_reason(format!(
                                "struct() expects a plain object, got '{}' instance",
                                name
                            )));
                        }
                    }
                }
            }
        }

        let mut fields = HashMap::new();
        for key in Object::keys(&object)? {
            if key.parse::<u32>().is_ok() {
                return Err(napi::Error::from_reason(
                    "Struct field names must not be numeric indices".to_string(),
                ));
            }
            let raw = object.get_named_property_unchecked::<Unknown>(&key)?;
            let value = Unknown::to_napi_value(env, raw)?;
            fields.insert(key, Value::from_napi_value(env, value)?);
        }

        Ok(PlainJsObject { fields })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    U32(u32),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    SparseVector(SparseVector),
    Bytes(Vec<u8>),
    List(List),
    Matrix(Matrix),
    Struct(HashMap<String, Value>),
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::I64(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::F64(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<Vec<u8>> for Value {
    fn from(values: Vec<u8>) -> Self {
        Value::List(List {
            values: Values::U8(values),
        })
    }
}

impl From<Vec<u32>> for Value {
    fn from(values: Vec<u32>) -> Self {
        Value::List(List {
            values: Values::U32(values),
        })
    }
}

impl From<Vec<u64>> for Value {
    fn from(values: Vec<u64>) -> Self {
        Value::List(List {
            values: Values::U64(values),
        })
    }
}

impl From<Vec<i8>> for Value {
    fn from(values: Vec<i8>) -> Self {
        Value::List(List {
            values: Values::I8(values),
        })
    }
}

impl From<Vec<i32>> for Value {
    fn from(values: Vec<i32>) -> Self {
        Value::List(List {
            values: Values::I32(values),
        })
    }
}

impl From<Vec<i64>> for Value {
    fn from(values: Vec<i64>) -> Self {
        Value::List(List {
            values: Values::I64(values),
        })
    }
}

impl From<Vec<f32>> for Value {
    fn from(values: Vec<f32>) -> Self {
        Value::List(List {
            values: Values::F32(values),
        })
    }
}

impl From<Vec<f64>> for Value {
    fn from(values: Vec<f64>) -> Self {
        Value::List(List {
            values: Values::F64(values),
        })
    }
}

impl From<Vec<String>> for Value {
    fn from(values: Vec<String>) -> Self {
        Value::List(List {
            values: Values::String(values),
        })
    }
}

impl From<Value> for topk_rs::proto::v1::data::Value {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => topk_rs::proto::v1::data::Value::null(),
            Value::Bool(b) => topk_rs::proto::v1::data::Value::bool(b),
            Value::String(s) => topk_rs::proto::v1::data::Value::string(s),
            Value::U32(n) => topk_rs::proto::v1::data::Value::u32(n),
            Value::I32(n) => topk_rs::proto::v1::data::Value::i32(n),
            Value::I64(n) => topk_rs::proto::v1::data::Value::i64(n),
            Value::F32(n) => topk_rs::proto::v1::data::Value::f32(n),
            Value::F64(n) => topk_rs::proto::v1::data::Value::f64(n),
            Value::Bytes(b) => topk_rs::proto::v1::data::Value::binary(b),
            Value::SparseVector(v) => v.into(),
            Value::List(v) => match v.values {
                Values::U8(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::U32(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::U64(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::I8(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::I32(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::I64(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::F8(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::F16(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::F32(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::F64(v) => topk_rs::proto::v1::data::Value::list(v),
                Values::String(v) => topk_rs::proto::v1::data::Value::list(v),
            },
            Value::Matrix(m) => m.into(),
            Value::Struct(fields) => topk_rs::proto::v1::data::Value::r#struct(
                fields.into_iter().map(|(k, v)| (k, v.into())),
            ),
        }
    }
}

impl From<topk_rs::proto::v1::data::Value> for Value {
    fn from(value: topk_rs::proto::v1::data::Value) -> Self {
        match value.value {
            // Null
            Some(topk_rs::proto::v1::data::value::Value::Null(_)) => Value::Null,
            // Bool
            Some(topk_rs::proto::v1::data::value::Value::Bool(b)) => Value::Bool(b),
            // String
            Some(topk_rs::proto::v1::data::value::Value::String(s)) => Value::String(s),
            // Numbers
            Some(topk_rs::proto::v1::data::value::Value::F64(n)) => Value::F64(n),
            Some(topk_rs::proto::v1::data::value::Value::F32(n)) => Value::F64(n as f64),
            Some(topk_rs::proto::v1::data::value::Value::U32(n)) => {
                Value::U32(n.try_into().expect("U32 is lossy"))
            }
            Some(topk_rs::proto::v1::data::value::Value::U64(n)) => {
                Value::U32(n.try_into().expect("U32 is lossy"))
            }
            Some(topk_rs::proto::v1::data::value::Value::I32(n)) => Value::I32(n),
            Some(topk_rs::proto::v1::data::value::Value::I64(n)) => Value::I64(n),
            // Bytes
            Some(topk_rs::proto::v1::data::value::Value::Binary(b)) => Value::Bytes(b.into()),
            // Vectors(deprecated)
            Some(topk_rs::proto::v1::data::value::Value::Vector(v)) => match v.vector {
                Some(topk_rs::proto::v1::data::vector::Vector::Float(float_vector)) => {
                    Value::List(List {
                        values: Values::F32(
                            #[allow(deprecated)]
                            float_vector.values,
                        ),
                    })
                }
                Some(topk_rs::proto::v1::data::vector::Vector::Byte(byte_vector)) => {
                    Value::List(List {
                        values: Values::U8(
                            #[allow(deprecated)]
                            byte_vector.values,
                        ),
                    })
                }
                None => unreachable!("Invalid vector proto"),
            },
            // Sparse vectors
            Some(topk_rs::proto::v1::data::value::Value::SparseVector(sparse_vector)) => {
                Value::SparseVector(match sparse_vector.values {
                    Some(topk_rs::proto::v1::data::sparse_vector::Values::F32(values)) => {
                        SparseVector::float(SparseVectorData::<f32> {
                            indices: sparse_vector.indices,
                            values: values.values,
                        })
                    }
                    Some(topk_rs::proto::v1::data::sparse_vector::Values::U8(values)) => {
                        SparseVector::byte(SparseVectorData::<u8> {
                            indices: sparse_vector.indices,
                            values: values.values,
                        })
                    }
                    // TODO: Implement F16, F8, I8 sparse vectors in js sdk
                    Some(topk_rs::proto::v1::data::sparse_vector::Values::F16(_)) => {
                        unimplemented!()
                    }
                    Some(topk_rs::proto::v1::data::sparse_vector::Values::F8(_)) => {
                        unimplemented!()
                    }
                    Some(topk_rs::proto::v1::data::sparse_vector::Values::I8(_)) => {
                        unimplemented!()
                    }
                    None => unreachable!("Invalid sparse vector proto"),
                })
            }
            Some(topk_rs::proto::v1::data::value::Value::List(list)) => Value::List(List {
                values: match list.values {
                    Some(topk_rs::proto::v1::data::list::Values::U8(v)) => Values::U8(v.values),
                    Some(topk_rs::proto::v1::data::list::Values::U32(v)) => Values::U32(v.values),
                    Some(topk_rs::proto::v1::data::list::Values::U64(v)) => Values::U64(v.values),
                    // Transmuting to i8 from the `bytes` u8 representation in proto
                    Some(topk_rs::proto::v1::data::list::Values::I8(v)) => Values::I8(v.into()),
                    Some(topk_rs::proto::v1::data::list::Values::I32(v)) => Values::I32(v.values),
                    Some(topk_rs::proto::v1::data::list::Values::I64(v)) => Values::I64(v.values),
                    Some(topk_rs::proto::v1::data::list::Values::F8(v)) => Values::F8(v.into()),
                    Some(topk_rs::proto::v1::data::list::Values::F16(v)) => Values::F16(v.into()),
                    Some(topk_rs::proto::v1::data::list::Values::F32(v)) => Values::F32(v.values),
                    Some(topk_rs::proto::v1::data::list::Values::F64(v)) => Values::F64(v.values),
                    Some(topk_rs::proto::v1::data::list::Values::String(v)) => {
                        Values::String(v.values)
                    }
                    None => unreachable!("Invalid list proto"),
                },
            }),
            Some(topk_rs::proto::v1::data::value::Value::Matrix(matrix)) => {
                let num_cols = matrix.num_cols;
                let values = match matrix.values {
                    Some(topk_rs::proto::v1::data::matrix::Values::F32(v)) => {
                        MatrixValues::F32(v.values)
                    }
                    Some(topk_rs::proto::v1::data::matrix::Values::F16(v)) => {
                        MatrixValues::F16(v.into())
                    }
                    Some(topk_rs::proto::v1::data::matrix::Values::F8(v)) => {
                        MatrixValues::F8(v.into())
                    }
                    Some(topk_rs::proto::v1::data::matrix::Values::U8(v)) => {
                        MatrixValues::U8(v.values)
                    }
                    Some(topk_rs::proto::v1::data::matrix::Values::I8(v)) => {
                        MatrixValues::I8(v.into())
                    }
                    None => unreachable!("Invalid matrix proto"),
                };

                Value::Matrix(Matrix { num_cols, values })
            }
            Some(topk_rs::proto::v1::data::value::Value::Struct(s)) => {
                Value::Struct(s.fields.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
            None => unreachable!("Invalid value proto"),
        }
    }
}

impl FromNapiValue for Value {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(list) = crate::try_cast_ref!(env, value, List) {
            return Ok(Value::List(list.clone()));
        }

        if let Ok(matrix) = crate::try_cast_ref!(env, value, Matrix) {
            return Ok(Value::Matrix(matrix.clone()));
        }

        if let Ok(sparse_vector) = crate::try_cast_ref!(env, value, SparseVector) {
            return Ok(Value::SparseVector(sparse_vector.clone()));
        }

        if let Ok(struct_value) = crate::try_cast_ref!(env, value, Struct) {
            return Ok(Value::Struct(struct_value.fields.clone()));
        }

        let mut result: i32 = 0;
        check_status!(napi::sys::napi_typeof(env, value, &mut result))?;
        match result {
            napi::sys::ValueType::napi_undefined => Ok(Value::Null),
            napi::sys::ValueType::napi_null => Ok(Value::Null),
            napi::sys::ValueType::napi_string => {
                Ok(Value::String(String::from_napi_value(env, value)?))
            }
            napi::sys::ValueType::napi_number => match is_napi_integer(env, value) {
                true => Ok(Value::I64(i64::from_napi_value(env, value)?)),
                false => Ok(Value::F64(f64::from_napi_value(env, value)?)),
            },
            napi::sys::ValueType::napi_boolean => {
                Ok(Value::Bool(bool::from_napi_value(env, value)?))
            }
            napi::sys::ValueType::napi_object => {
                // Number lists (all "naked" number lists are interpreted as f32 lists (casting from f64))
                if let Ok(list) = Vec::<f64>::from_napi_value(env, value) {
                    return Ok(Value::List(List {
                        values: Values::F32(list.into_iter().map(|v| v as f32).collect()),
                    }));
                }

                // Matrices (all "naked" array of arrays are interpreted as f32 matrices)
                if let Ok(matrix_rows) = Vec::<Vec<f64>>::from_napi_value(env, value) {
                    return Ok(Value::Matrix(Matrix::from_list_of_lists(
                        matrix_rows,
                        Some(MatrixValueType::F32),
                    )?));
                }

                // List of strings
                if let Ok(list) = Vec::<String>::from_napi_value(env, value) {
                    return Ok(Value::List(List {
                        values: Values::String(list),
                    }));
                }

                // Bytes/buffers
                if let Ok(bytes) = from_napi_buffer(env, value) {
                    return Ok(Value::Bytes(bytes.into()));
                }

                // Arrays of objects/mixed types slip past the f32/string list checks above.
                let mut is_array = false;
                check_status!(napi::sys::napi_is_array(env, value, &mut is_array))?;
                if is_array {
                    return Err(napi::Error::from_reason(
                        "Arrays are not valid struct values; use a typed list constructor instead"
                            .to_string(),
                    ));
                }

                let object = Object::from_napi_value(env, value)?;

                // Must precede sparse vector check: objects with no enumerable keys (e.g. Date)
                // would otherwise become empty sparse vectors.
                if let Ok(ctor) = object.get_named_property::<Unknown>("constructor") {
                    let ctor_napi = Unknown::to_napi_value(env, ctor)?;
                    let mut ctor_type: i32 = 0;
                    check_status!(napi::sys::napi_typeof(env, ctor_napi, &mut ctor_type))?;
                    if ctor_type == napi::sys::ValueType::napi_function {
                        if let Ok(ctor_obj) = Object::from_napi_value(env, ctor_napi) {
                            if let Ok(name) = ctor_obj.get_named_property::<String>("name") {
                                if name != "Object" {
                                    return Err(napi::Error::from_reason(format!(
                                        "Unsupported object type '{}'",
                                        name
                                    )));
                                }
                            }
                        }
                    }
                }

                let keys = Object::keys(&object)?;

                // Sparse vectors (all "naked" sparse vectors are interpreted as f32).
                // Empty objects are structs — skip the check so {} doesn't become SparseVector([]).
                if !keys.is_empty() {
                    if let Ok(sparse_vector) =
                        SparseVectorData::<f64>::from_napi_value(env, value)
                    {
                        return Ok(Value::SparseVector(SparseVector::float(
                            sparse_vector
                                .into_iter()
                                .map(|(i, v)| (i, v as f32))
                                .collect(),
                        )));
                    }
                }

                let mut fields = HashMap::new();

                for key in keys {
                    if key.parse::<u32>().is_ok() {
                        return Err(napi::Error::from_reason(
                            "Struct field names must not be numeric indices".to_string(),
                        ));
                    }

                    let raw = object.get_named_property_unchecked::<Unknown>(&key)?;
                    let value = Unknown::to_napi_value(env, raw)?;
                    fields.insert(key, Value::from_napi_value(env, value)?);
                }

                return Ok(Value::Struct(fields));
            }
            _ => Err(napi::Error::from_reason(format!(
                "Unsupported napi value type: {:?}",
                value
            ))),
        }
    }
}

impl ToNapiValue for Value {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value> {
        match val {
            Value::String(s) => String::to_napi_value(env, s),
            Value::F64(n) => f64::to_napi_value(env, n),
            Value::Bool(b) => bool::to_napi_value(env, b),
            Value::U32(n) => u32::to_napi_value(env, n),
            Value::I32(n) => i32::to_napi_value(env, n),
            Value::I64(n) => i64::to_napi_value(env, n),
            Value::F32(n) => f32::to_napi_value(env, n),
            Value::Bytes(b) => Buffer::to_napi_value(env, b.into()),
            Value::SparseVector(v) => match v.0 {
                SparseVectorUnion::Float { vector } => {
                    SparseVectorData::<f32>::to_napi_value(env, vector)
                }
                SparseVectorUnion::Byte { vector } => {
                    SparseVectorData::<u8>::to_napi_value(env, vector)
                }
            },
            Value::Null => Null::to_napi_value(env, Null),
            Value::List(v) => match v.values {
                Values::U8(v) => Vec::<u8>::to_napi_value(env, v),
                Values::U32(v) => Vec::<u32>::to_napi_value(env, v),
                Values::U64(v) => {
                    Vec::<u32>::to_napi_value(env, v.iter().map(|v| *v as u32).collect())
                }
                Values::I8(v) => Vec::<i8>::to_napi_value(env, v),
                Values::I32(v) => Vec::<i32>::to_napi_value(env, v),
                Values::I64(v) => Vec::<i64>::to_napi_value(env, v),
                Values::F8(v) => {
                    Vec::<f32>::to_napi_value(env, v.iter().map(|x| x.to_f32()).collect())
                }
                Values::F16(v) => {
                    Vec::<f32>::to_napi_value(env, v.iter().map(|x| x.to_f32()).collect())
                }
                Values::F32(v) => Vec::<f32>::to_napi_value(env, v),
                Values::F64(v) => Vec::<f64>::to_napi_value(env, v),
                Values::String(v) => Vec::<String>::to_napi_value(env, v),
            },
            Value::Matrix(m) => {
                // Always convert to nested JS arrays of numbers (`number[][]`).
                let num_cols = m.num_cols as usize;
                let rows: Vec<Vec<f64>> = match m.values {
                    MatrixValues::F32(v) => v
                        .chunks(num_cols)
                        .map(|row| row.iter().map(|x| *x as f64).collect())
                        .collect(),
                    MatrixValues::F16(v) => v
                        .chunks(num_cols)
                        .map(|row| row.iter().map(|x| x.to_f32() as f64).collect())
                        .collect(),
                    MatrixValues::F8(v) => v
                        .chunks(num_cols)
                        .map(|row| row.iter().map(|x| x.to_f32() as f64).collect())
                        .collect(),
                    MatrixValues::U8(v) => v
                        .chunks(num_cols)
                        .map(|row| row.iter().map(|x| *x as f64).collect())
                        .collect(),
                    MatrixValues::I8(v) => v
                        .chunks(num_cols)
                        .map(|row| row.iter().map(|x| *x as f64).collect())
                        .collect(),
                };

                Vec::<Vec<f64>>::to_napi_value(env, rows)
            }
            Value::Struct(fields) => {
                let mut object = ptr::null_mut();
                check_status!(
                    napi::sys::napi_create_object(env, &mut object),
                    "Failed to create JavaScript object"
                )?;

                for (key, value) in fields {
                    let key = CString::new(key).map_err(|_| {
                        napi::Error::from_reason(
                            "Struct field name contains an embedded null byte".to_string(),
                        )
                    })?;
                    let value = Value::to_napi_value(env, value)?;

                    check_status!(
                        napi::sys::napi_set_named_property(env, object, key.as_ptr(), value),
                        "Failed to set property"
                    )?;
                }

                Ok(object)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct NativeValue(pub(crate) Value);

impl ToNapiValue for NativeValue {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value> {
        Value::to_napi_value(env, val.0)
    }
}

impl From<topk_rs::proto::v1::data::Value> for NativeValue {
    fn from(value: topk_rs::proto::v1::data::Value) -> Self {
        NativeValue(Value::from(value))
    }
}

unsafe fn is_napi_integer(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> bool {
    // Check if the number is an integer by comparing it with its integer part
    let num = f64::from_napi_value(env, napi_val).unwrap();
    num == (num as i64) as f64
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BytesData(pub(crate) Vec<u8>);

impl Into<Vec<u8>> for BytesData {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

impl FromIterator<u8> for BytesData {
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Self {
        BytesData(iter.into_iter().collect())
    }
}

impl ToNapiValue for BytesData {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        Vec::<u8>::to_napi_value(env, val.0)
    }
}

impl FromNapiValue for BytesData {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(array) = Vec::<u8>::from_napi_value(env, value) {
            return Ok(BytesData(array));
        }

        if let Ok(bytes) = from_napi_buffer(env, value) {
            return Ok(bytes);
        }

        Err(napi::Error::from_reason(
            "Invalid bytes value, must be `number[]` or `Buffer`",
        ))
    }
}

unsafe fn from_napi_buffer(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
) -> napi::Result<BytesData> {
    // To prevent panics on invalid buffer values such as bytes([-1])
    // we check if the value is a JS buffer
    let mut is_js_buffer: bool = false;
    napi_is_buffer(env, value, &mut is_js_buffer);

    if is_js_buffer {
        if let Ok(buffer) = Buffer::from_napi_value(env, value) {
            return Ok(BytesData(buffer.to_vec()));
        }
    }

    Err(napi::Error::from_reason("Could not parse buffer object"))
}
