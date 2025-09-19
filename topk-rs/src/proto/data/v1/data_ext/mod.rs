use crate::proto::data::v1::list;

mod document;
mod sparse_vector;
mod value;
use std::mem::{align_of, size_of, ManuallyDrop};

pub trait IntoListValues {
    fn into_list_values(self) -> list::Values;
}

impl IntoListValues for Vec<u8> {
    fn into_list_values(self) -> list::Values {
        list::Values::U8(list::U8 { values: self })
    }
}

impl IntoListValues for Vec<u32> {
    fn into_list_values(self) -> list::Values {
        list::Values::U32(list::U32 { values: self })
    }
}

impl IntoListValues for Vec<u64> {
    fn into_list_values(self) -> list::Values {
        list::Values::U64(list::U64 { values: self })
    }
}

impl IntoListValues for Vec<i8> {
    fn into_list_values(self) -> list::Values {
        // Transmute to u8 and use `bytes` in proto because it doesn't have an i8 type
        let u8_vec = transmute_vec_i8_to_u8(self);
        list::Values::I8(list::I8 { values: u8_vec })
    }
}

impl IntoListValues for Vec<i32> {
    fn into_list_values(self) -> list::Values {
        list::Values::I32(list::I32 { values: self })
    }
}

impl IntoListValues for Vec<i64> {
    fn into_list_values(self) -> list::Values {
        list::Values::I64(list::I64 { values: self })
    }
}

impl IntoListValues for Vec<f32> {
    fn into_list_values(self) -> list::Values {
        list::Values::F32(list::F32 { values: self })
    }
}

impl IntoListValues for Vec<f64> {
    fn into_list_values(self) -> list::Values {
        list::Values::F64(list::F64 { values: self })
    }
}

impl IntoListValues for Vec<String> {
    fn into_list_values(self) -> list::Values {
        list::Values::String(list::String { values: self })
    }
}

fn transmute_vec_i8_to_u8(vec: Vec<i8>) -> Vec<u8> {
    debug_assert_eq!(size_of::<i8>(), size_of::<u8>());
    debug_assert_eq!(align_of::<i8>(), align_of::<u8>());

    let mut v = ManuallyDrop::new(vec);
    let ptr = v.as_mut_ptr() as *mut u8;
    let len = v.len();
    let cap = v.capacity();
    // SAFETY:
    //  - size and alignment of u8 and i8 is the same
    //  - double free is prevented by ManuallyDrop
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

fn transmute_vec_u8_to_i8(vec: Vec<u8>) -> Vec<i8> {
    debug_assert_eq!(size_of::<i8>(), size_of::<u8>());
    debug_assert_eq!(align_of::<i8>(), align_of::<u8>());

    let mut v = ManuallyDrop::new(vec);
    let ptr = v.as_mut_ptr() as *mut i8;
    let len = v.len();
    let cap = v.capacity();
    // SAFETY:
    //  - size and alignment of u8 and i8 is the same
    //  - double free is prevented by ManuallyDrop
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}
