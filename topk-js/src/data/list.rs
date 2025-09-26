use napi_derive::napi;
use topk_rs::proto::v1::data::IntoListValues;

/// @internal
/// @hideconstructor
/// Instances of the `List` class are used to represent lists of values in TopK.
/// Usually created using data constructors such as [`f32_list()`](#f32list), [`i32_list()`](#i32list), etc.
#[derive(Debug, Clone, PartialEq)]
#[napi(namespace = "data")]
pub struct List {
    pub(crate) values: Values,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Values {
    U8(Vec<u8>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    I8(Vec<i8>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    String(Vec<String>),
}

/// @internal
#[napi(namespace = "data")]
impl List {
    /// @ignore
    #[napi]
    pub fn to_string(&self) -> String {
        format!("List({:?})", self.values)
    }
}

impl From<List> for topk_rs::proto::v1::data::List {
    fn from(list: List) -> Self {
        match list.values {
            Values::U8(values) => topk_rs::proto::v1::data::List {
                values: Some(values.into_list_values()),
            },
            Values::U32(values) => topk_rs::proto::v1::data::List {
                values: Some(values.into_list_values()),
            },
            Values::U64(values) => topk_rs::proto::v1::data::List {
                values: Some(values.into_list_values()),
            },
            Values::I8(values) => topk_rs::proto::v1::data::List {
                values: Some(values.into_list_values()),
            },
            Values::I32(values) => topk_rs::proto::v1::data::List {
                values: Some(values.into_list_values()),
            },
            Values::I64(values) => topk_rs::proto::v1::data::List {
                values: Some(values.into_list_values()),
            },
            Values::F32(values) => topk_rs::proto::v1::data::List {
                values: Some(values.into_list_values()),
            },
            Values::F64(values) => topk_rs::proto::v1::data::List {
                values: Some(values.into_list_values()),
            },
            Values::String(values) => topk_rs::proto::v1::data::List {
                values: Some(values.into_list_values()),
            },
        }
    }
}
