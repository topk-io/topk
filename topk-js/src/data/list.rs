use napi_derive::napi;

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
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    String(Vec<String>),
}

#[napi(namespace = "data")]
impl List {
    #[napi]
    pub fn to_string(&self) -> String {
        format!("List({:?})", self.values)
    }
}

impl From<List> for topk_rs::proto::v1::data::List {
    fn from(list: List) -> Self {
        match list.values {
            Values::U8(values) => topk_rs::proto::v1::data::List {
                values: Some(topk_rs::proto::v1::data::list::Values::U8(
                    topk_rs::proto::v1::data::list::U8 { values },
                )),
            },
            Values::U32(values) => topk_rs::proto::v1::data::List {
                values: Some(topk_rs::proto::v1::data::list::Values::U32(
                    topk_rs::proto::v1::data::list::U32 { values },
                )),
            },
            Values::U64(values) => topk_rs::proto::v1::data::List {
                values: Some(topk_rs::proto::v1::data::list::Values::U64(
                    topk_rs::proto::v1::data::list::U64 { values },
                )),
            },
            Values::I32(values) => topk_rs::proto::v1::data::List {
                values: Some(topk_rs::proto::v1::data::list::Values::I32(
                    topk_rs::proto::v1::data::list::I32 { values },
                )),
            },
            Values::I64(values) => topk_rs::proto::v1::data::List {
                values: Some(topk_rs::proto::v1::data::list::Values::I64(
                    topk_rs::proto::v1::data::list::I64 { values },
                )),
            },
            Values::F32(values) => topk_rs::proto::v1::data::List {
                values: Some(topk_rs::proto::v1::data::list::Values::F32(
                    topk_rs::proto::v1::data::list::F32 { values },
                )),
            },
            Values::F64(values) => topk_rs::proto::v1::data::List {
                values: Some(topk_rs::proto::v1::data::list::Values::F64(
                    topk_rs::proto::v1::data::list::F64 { values },
                )),
            },
            Values::String(values) => topk_rs::proto::v1::data::List {
                values: Some(topk_rs::proto::v1::data::list::Values::String(
                    topk_rs::proto::v1::data::list::String { values },
                )),
            },
        }
    }
}
