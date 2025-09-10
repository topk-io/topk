use std::collections::HashMap;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator,
    __S::Error: rkyv::rancor::Source,
))]
#[rkyv(deserialize_bounds(__D::Error: rkyv::rancor::Source))]
#[rkyv(bytecheck(bounds(__C: rkyv::validation::ArchiveContext)))]
#[repr(C, u8)]
pub enum Value {
    // Boolean
    Bool(bool),
    // Unsigned
    U32(u32),
    U64(u64),
    // Signed
    I32(i32),
    I64(i64),
    // Floating point
    F32(f32),
    F64(f64),
    // String
    String(String),
    // Binary
    Binary(Vec<u8>),
    // Sparse vector
    SparseVector(SparseVector),
    // List
    List(ListValue),
    // Struct
    Struct(#[rkyv(omit_bounds)] StructValue),
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[repr(C)]
pub struct StructValue {
    pub fields: HashMap<String, Value>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[repr(C, u8)]
pub enum ListValue {
    // Unsigned
    U8(Vec<u8>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    // Signed
    I8(Vec<i8>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    // Floating point
    F32(Vec<f32>),
    F64(Vec<f64>),
    // String
    String(Vec<String>),
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[repr(C)]
pub struct SparseVector {
    pub indices: Vec<u32>,
    pub values: ListValue,
}
