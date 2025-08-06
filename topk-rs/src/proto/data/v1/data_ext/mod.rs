use crate::proto::data::v1::list;

mod document;
mod sparse_vector;
mod value;

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
