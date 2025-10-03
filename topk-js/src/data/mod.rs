mod collection;
pub use collection::Collection;

mod list;
pub use list::List;
pub use list::Values;

mod scalar;
pub use scalar::Scalar;

mod value;
pub use value::NativeValue;
pub use value::Value;

mod vector;
pub use vector::SparseVector;
use vector::SparseVectorData;

use napi_derive::napi;
use value::BytesData;

/// Creates a bytes value from a buffer or array of numbers.
#[napi(namespace = "data", ts_return_type = "Buffer")]
pub fn bytes(#[napi(ts_arg_type = "Array<number> | Buffer")] buffer: BytesData) -> Value {
    Value::Bytes(buffer.into())
}

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing a 32-bit float vector. This function is an alias for [f32List()](https://docs.topk.io/sdk/topk-js/data#f32list).
///
/// Example:
///
/// ```javascript
/// import { f32Vector } from "topk-js/data";
///
/// f32Vector([0.12, 0.67, 0.82, 0.53])
/// ```
#[napi(namespace = "data")]
pub fn f32_vector(values: Vec<f64>) -> List {
    List {
        values: Values::F32(values.into_iter().map(|v| v as f32).collect()),
    }
}

/// Creates an 8-bit unsigned integer vector from an array of numbers.
#[napi(namespace = "data")]
pub fn u8_vector(values: Vec<u8>) -> List {
    List {
        values: Values::U8(values),
    }
}

/// Creates an 8-bit signed integer vector from an array of numbers.
#[napi(namespace = "data")]
pub fn i8_vector(values: Vec<i8>) -> List {
    List {
        values: Values::I8(values),
    }
}

/// Creates a binary vector from an array of bytes.
#[napi(namespace = "data")]
pub fn binary_vector(values: Vec<u8>) -> List {
    List {
        values: Values::U8(values),
    }
}

/// Creates a list of 32-bit unsigned integers.
#[napi(namespace = "data")]
pub fn u32_list(values: Vec<u32>) -> List {
    List {
        values: Values::U32(values),
    }
}

/// Creates a list of 32-bit signed integers.
#[napi(namespace = "data")]
pub fn i32_list(values: Vec<i32>) -> List {
    List {
        values: Values::I32(values),
    }
}

/// Creates a list of 64-bit signed integers.
#[napi(namespace = "data")]
pub fn i64_list(values: Vec<i64>) -> List {
    List {
        values: Values::I64(values),
    }
}

/// Creates a list of 64-bit floating point numbers.
#[napi(namespace = "data")]
pub fn f32_list(values: Vec<f64>) -> List {
    List {
        values: Values::F32(values.into_iter().map(|v| v as f32).collect()),
    }
}

#[napi(namespace = "data")]
pub fn f64_list(values: Vec<f64>) -> List {
    List {
        values: Values::F64(values),
    }
}

/// Creates a sparse vector of 32-bit floats.
#[napi(namespace = "data")]
pub fn string_list(values: Vec<String>) -> List {
    List {
        values: Values::String(values),
    }
}

#[napi(namespace = "data")]
pub fn f32_sparse_vector(
    #[napi(ts_arg_type = "Record<number, number>")] vector: SparseVectorData<f64>,
) -> SparseVector {
    SparseVector::float(vector.into_iter().map(|(i, v)| (i, v as f32)).collect())
}

/// Creates a sparse vector of 8-bit unsigned integers.
#[napi(namespace = "data")]
pub fn u8_sparse_vector(
    #[napi(ts_arg_type = "Record<number, number>")] vector: SparseVectorData<u8>,
) -> SparseVector {
    SparseVector::byte(vector)
}
