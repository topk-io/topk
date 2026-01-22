mod collection;
pub use collection::Collection;

mod list;
pub use list::List;
pub use list::Values;

mod matrix;
pub use matrix::Matrix;
pub use matrix::MatrixValueType;

mod value;
pub use value::NativeValue;
pub use value::Value;

mod vector;
pub use vector::SparseVector;
use vector::SparseVectorData;

use napi_derive::napi;
use value::BytesData;

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing bytes data.
///
/// Example:
///
/// ```javascript
/// import { bytes } from "topk-js/data";
///
/// bytes([0, 1, 1, 0])
/// ```
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

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing an 8-bit unsigned integer vector. This function is an alias for [u8List()](https://docs.topk.io/sdk/topk-js/data#u8list).
///
/// Example:
///
/// ```javascript
/// import { u8Vector } from "topk-js/data";
///
/// u8Vector([0, 255, 1, 2, 3])
/// ```
#[napi(namespace = "data")]
pub fn u8_vector(values: Vec<u8>) -> List {
    List {
        values: Values::U8(values),
    }
}

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing an 8-bit signed integer vector. This function is an alias for [i8List()](https://docs.topk.io/sdk/topk-js/data#i8list).
///
/// Example:
///
/// ```javascript
/// import { i8Vector } from "topk-js/data";
///
/// i8Vector([-128, 127, -1, 0, 1])
/// ```
#[napi(namespace = "data")]
pub fn i8_vector(values: Vec<i8>) -> List {
    List {
        values: Values::I8(values),
    }
}

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing a binary vector. This function is an alias for [binaryList()](https://docs.topk.io/sdk/topk-js/data#binarylist).
///
/// Example:
///
/// ```javascript
/// import { binaryVector } from "topk-js/data";
///
/// binaryVector([0, 1, 1, 0])
/// ```
#[napi(namespace = "data")]
pub fn binary_vector(values: Vec<u8>) -> List {
    List {
        values: Values::U8(values),
    }
}

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing a list of 32-bit unsigned integers.
///
/// Example:
///
/// ```javascript
/// import { u32List } from "topk-js/data";
///
/// u32List([0, 1, 2, 3])
/// ```
#[napi(namespace = "data")]
pub fn u32_list(values: Vec<u32>) -> List {
    List {
        values: Values::U32(values),
    }
}

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing a list of 32-bit signed integers.
///
/// Example:
///
/// ```javascript
/// import { i32List } from "topk-js/data";
///
/// i32List([0, 1, 2, 3])
/// ```
#[napi(namespace = "data")]
pub fn i32_list(values: Vec<i32>) -> List {
    List {
        values: Values::I32(values),
    }
}

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing a list of 64-bit signed integers.
///
/// Example:
///
/// ```javascript
/// import { i64List } from "topk-js/data";
///
/// i64List([0, 1, 2, 3])
/// ```
#[napi(namespace = "data")]
pub fn i64_list(values: Vec<i64>) -> List {
    List {
        values: Values::I64(values),
    }
}

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing a list of 32-bit floating point numbers.
///
/// Example:
///
/// ```javascript
/// import { f32List } from "topk-js/data";
///
/// f32List([0.12, 0.67, 0.82, 0.53])
/// ```
#[napi(namespace = "data")]
pub fn f32_list(values: Vec<f64>) -> List {
    List {
        values: Values::F32(values.into_iter().map(|v| v as f32).collect()),
    }
}

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing a list of 64-bit floating point numbers.
///
/// Example:
///
/// ```javascript
/// import { f64List } from "topk-js/data";
///
/// f64List([0.12, 0.67, 0.82, 0.53])
/// ```
#[napi(namespace = "data")]
pub fn f64_list(values: Vec<f64>) -> List {
    List {
        values: Values::F64(values),
    }
}

/// Creates a [List](https://docs.topk.io/sdk/topk-js/data#List) type containing a list of strings.
///
/// Example:
///
/// ```javascript
/// import { stringList } from "topk-js/data";
///
/// stringList(["foo", "bar", "baz"])
/// ```
#[napi(namespace = "data")]
pub fn string_list(values: Vec<String>) -> List {
    List {
        values: Values::String(values),
    }
}

/// Create a [Matrix](https://docs.topk.io/sdk/topk-js/data#Matrix) type containing matrix values.
///
/// The `values` parameter must be an array of number arrays. When passing an array of number arrays,
/// the optional `valueType` parameter specifies the matrix type.
/// If `valueType` is not provided, the matrix defaults to f32.
///
/// ```javascript
/// import { matrix } from "topk-js/data";
///
/// // Array of number arrays with explicit type
/// matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]], "f32")
///
/// // Array of number arrays defaults to f32
/// matrix([[1.0, 2.0], [3.0, 4.0]])
/// ```
#[napi(namespace = "data")]
pub fn matrix(values: Vec<Vec<f64>>, value_type: Option<MatrixValueType>) -> napi::Result<Matrix> {
    Matrix::from_list_of_lists(values, value_type)
}

/// Creates a [SparseVector](https://docs.topk.io/sdk/topk-js/data#SparseVector) type containing a sparse vector of 32-bit floats. This function is an alias for [f32SparseList()](https://docs.topk.io/sdk/topk-js/data#f32sparselist).
///
/// Example:
///
/// ```javascript
/// import { f32SparseVector } from "topk-js/data";
///
/// f32SparseVector({0: 0.12, 6: 0.67, 17: 0.82, 97: 0.53})
/// ```
#[napi(namespace = "data")]
pub fn f32_sparse_vector(
    #[napi(ts_arg_type = "Record<number, number>")] vector: SparseVectorData<f64>,
) -> SparseVector {
    SparseVector::float(vector.into_iter().map(|(i, v)| (i, v as f32)).collect())
}

/// Creates a [SparseVector](https://docs.topk.io/sdk/topk-js/data#SparseVector) type containing a sparse vector of 8-bit unsigned integers. This function is an alias for [u8SparseList()](https://docs.topk.io/sdk/topk-js/data#u8sparselist).
///
/// Example:
///
/// ```javascript
/// import { u8SparseVector } from "topk-js/data";
///
/// u8SparseVector({0: 12, 6: 67, 17: 82, 97: 53})
/// ```
#[napi(namespace = "data")]
pub fn u8_sparse_vector(
    #[napi(ts_arg_type = "Record<number, number>")] vector: SparseVectorData<u8>,
) -> SparseVector {
    SparseVector::byte(vector)
}
