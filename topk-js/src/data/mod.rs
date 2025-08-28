mod collection;
pub use collection::{Collection, CollectionFieldSpec};

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

#[napi(namespace = "data")]
pub fn bytes(#[napi(ts_arg_type = "Array<number> | Buffer")] buffer: BytesData) -> Value {
    Value::Bytes(buffer.into())
}

#[napi(namespace = "data")]
pub fn f32_vector(values: Vec<f64>) -> List {
    List {
        values: Values::F32(values.into_iter().map(|v| v as f32).collect()),
    }
}

#[napi(namespace = "data")]
pub fn u8_vector(values: Vec<u8>) -> List {
    List {
        values: Values::U8(values),
    }
}

#[napi(namespace = "data")]
pub fn binary_vector(values: Vec<u8>) -> List {
    List {
        values: Values::U8(values),
    }
}

#[napi(namespace = "data")]
pub fn u32_list(values: Vec<u32>) -> List {
    List {
        values: Values::U32(values),
    }
}

#[napi(namespace = "data")]
pub fn i32_list(values: Vec<i32>) -> List {
    List {
        values: Values::I32(values),
    }
}

#[napi(namespace = "data")]
pub fn i64_list(values: Vec<i64>) -> List {
    List {
        values: Values::I64(values),
    }
}

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

#[napi(namespace = "data")]
pub fn u8_sparse_vector(
    #[napi(ts_arg_type = "Record<number, number>")] vector: SparseVectorData<u8>,
) -> SparseVector {
    SparseVector::byte(vector)
}
