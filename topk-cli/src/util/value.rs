use topk_rs::proto::v1::data::{list, value, vector, List, Value};

pub fn value_to_json(value: Value) -> serde_json::Value {
    match value.value {
        Some(value::Value::Null(_)) => serde_json::Value::Null,
        Some(value::Value::Bool(v)) => serde_json::Value::Bool(v),
        Some(value::Value::String(v)) => serde_json::Value::String(v),
        Some(value::Value::U32(v)) => serde_json::Value::from(v),
        Some(value::Value::U64(v)) => serde_json::Value::from(v),
        Some(value::Value::I32(v)) => serde_json::Value::from(v),
        Some(value::Value::I64(v)) => serde_json::Value::from(v),
        Some(value::Value::F32(v)) => serde_json::Number::from_f64(v as f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Some(value::Value::F64(v)) => serde_json::Number::from_f64(v)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Some(value::Value::Binary(v)) => {
            serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect())
        }
        #[allow(deprecated)]
        Some(value::Value::Vector(v)) => match v.vector {
            Some(vector::Vector::Float(v)) => serde_json::Value::Array(
                v.values.into_iter().map(serde_json::Value::from).collect(),
            ),
            Some(vector::Vector::Byte(v)) => serde_json::Value::Array(
                v.values.into_iter().map(serde_json::Value::from).collect(),
            ),
            None => serde_json::Value::Null,
        },
        Some(value::Value::Struct(v)) => serde_json::Value::Object(
            v.fields
                .into_iter()
                .map(|(k, v)| (k, value_to_json(v)))
                .collect(),
        ),
        Some(value::Value::List(v)) => list_to_json(v),
        // Metadata should not contain sparse vectors or matrices, return null
        Some(value::Value::SparseVector(_)) | Some(value::Value::Matrix(_)) => {
            serde_json::Value::Null
        }
        None => serde_json::Value::Null,
    }
}

fn list_to_json(list: List) -> serde_json::Value {
    let values = match list.values {
        Some(list::Values::U32(v)) => v.values.into_iter().map(serde_json::Value::from).collect(),
        Some(list::Values::U64(v)) => v.values.into_iter().map(serde_json::Value::from).collect(),
        Some(list::Values::I32(v)) => v.values.into_iter().map(serde_json::Value::from).collect(),
        Some(list::Values::I64(v)) => v.values.into_iter().map(serde_json::Value::from).collect(),
        Some(list::Values::F8(v)) => v
            .as_ref()
            .iter()
            .map(|n| serde_json::Value::from(f32::from(*n)))
            .collect(),
        Some(list::Values::F16(v)) => v
            .as_ref()
            .iter()
            .map(|n| serde_json::Value::from(f32::from(*n)))
            .collect(),
        Some(list::Values::F32(v)) => v
            .values
            .into_iter()
            .map(|n| {
                serde_json::Number::from_f64(n as f64)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            })
            .collect(),
        Some(list::Values::F64(v)) => v
            .values
            .into_iter()
            .map(|n| {
                serde_json::Number::from_f64(n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            })
            .collect(),
        Some(list::Values::String(v)) => {
            v.values.into_iter().map(serde_json::Value::from).collect()
        }
        Some(list::Values::U8(v)) => v.values.into_iter().map(serde_json::Value::from).collect(),
        Some(list::Values::I8(v)) => v
            .as_ref()
            .iter()
            .copied()
            .map(serde_json::Value::from)
            .collect(),
        None => Vec::new(),
    };

    serde_json::Value::Array(values)
}
