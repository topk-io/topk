use crate::data::list::{List, Values};

#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    List(List),
}

impl From<i64> for Scalar {
    fn from(value: i64) -> Self {
        Scalar::I64(value)
    }
}

impl From<f64> for Scalar {
    fn from(value: f64) -> Self {
        Scalar::F64(value)
    }
}

impl From<bool> for Scalar {
    fn from(value: bool) -> Self {
        Scalar::Bool(value)
    }
}

impl From<String> for Scalar {
    fn from(value: String) -> Self {
        Scalar::String(value)
    }
}

impl From<Vec<u8>> for Scalar {
    fn from(values: Vec<u8>) -> Self {
        Scalar::List(List {
            values: Values::U8(values),
        })
    }
}

impl From<Vec<u32>> for Scalar {
    fn from(values: Vec<u32>) -> Self {
        Scalar::List(List {
            values: Values::U32(values),
        })
    }
}

impl From<Vec<u64>> for Scalar {
    fn from(values: Vec<u64>) -> Self {
        Scalar::List(List {
            values: Values::U64(values),
        })
    }
}

impl From<Vec<i8>> for Scalar {
    fn from(values: Vec<i8>) -> Self {
        Scalar::List(List {
            values: Values::I8(values),
        })
    }
}

impl From<Vec<i32>> for Scalar {
    fn from(values: Vec<i32>) -> Self {
        Scalar::List(List {
            values: Values::I32(values),
        })
    }
}

impl From<Vec<i64>> for Scalar {
    fn from(values: Vec<i64>) -> Self {
        Scalar::List(List {
            values: Values::I64(values),
        })
    }
}

impl From<Vec<f32>> for Scalar {
    fn from(values: Vec<f32>) -> Self {
        Scalar::List(List {
            values: Values::F32(values),
        })
    }
}

impl From<Vec<f64>> for Scalar {
    fn from(values: Vec<f64>) -> Self {
        Scalar::List(List {
            values: Values::F64(values),
        })
    }
}

impl From<Vec<String>> for Scalar {
    fn from(values: Vec<String>) -> Self {
        Scalar::List(List {
            values: Values::String(values),
        })
    }
}

impl Into<topk_rs::proto::v1::data::Value> for Scalar {
    fn into(self) -> topk_rs::proto::v1::data::Value {
        match self {
            Scalar::Bool(b) => topk_rs::proto::v1::data::Value::bool(b),
            Scalar::I64(i) => topk_rs::proto::v1::data::Value::i64(i),
            Scalar::F64(f) => topk_rs::proto::v1::data::Value::f64(f),
            Scalar::String(s) => topk_rs::proto::v1::data::Value::string(s),
            Scalar::List(l) => match l.values {
                Values::U8(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::U32(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::U64(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::I8(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::I32(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::I64(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::F32(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::F64(values) => topk_rs::proto::v1::data::Value::list(values),
                Values::String(values) => topk_rs::proto::v1::data::Value::list(values),
            },
        }
    }
}
