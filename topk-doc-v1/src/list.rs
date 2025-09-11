use rkyv::{Archive, Deserialize, Serialize};

use crate::ScalarType;

#[derive(Archive, Deserialize, Serialize, Clone, Debug, PartialEq)]
#[repr(C, u8)]
pub enum ListValue {
    // Unsigned
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    // Signed
    I8(Vec<i8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    // Floating point
    F32(Vec<f32>),
    F64(Vec<f64>),
    // String
    String(Vec<String>),
}

impl ListValue {
    pub fn as_slice<T: ScalarType>(&self) -> Option<&[T]> {
        T::as_slice(self)
    }
}

impl<T> From<Vec<T>> for ListValue
where
    T: ScalarType,
{
    fn from(value: Vec<T>) -> Self {
        T::to_list(value)
    }
}
