use crate::proto::data::v1::list;

mod document;
mod sparse_vector;
mod value;
use bytemuck::allocation::cast_vec;

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
        let u8_vec = cast_vec(self);
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
