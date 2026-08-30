use mongodb::bson::Bson as Wire;
use topk_rs::proto::v1::data::Value;

use crate::import::error::Error;
use crate::import::spec::Type;

pub fn ty(input: &Wire) -> Type {
    match input {
        Wire::Boolean(_) => Type::Bool,
        Wire::Int32(_) | Wire::Int64(_) => Type::Int,
        Wire::Double(_) => Type::Float,
        Wire::Decimal128(_) => Type::Text,
        Wire::Binary(_) => Type::Bytes,
        Wire::Document(_) => Type::Struct,
        Wire::Array(items) if items.is_empty() => Type::FloatList,
        Wire::Array(items) => {
            if items
                .iter()
                .all(|b| matches!(b, Wire::Int32(_) | Wire::Int64(_)))
            {
                Type::IntList
            } else if items.iter().all(|b| {
                matches!(
                    b,
                    Wire::Int32(_) | Wire::Int64(_) | Wire::Double(_) | Wire::Decimal128(_)
                )
            }) {
                Type::FloatList
            } else if items.iter().all(|b| matches!(b, Wire::String(_))) {
                Type::TextList
            } else {
                Type::Text
            }
        }
        _ => Type::Text,
    }
}

pub fn value(input: Wire) -> Result<Value, Error> {
    Ok(match input {
        Wire::Null | Wire::Undefined => Value::null(),
        Wire::Boolean(b) => Value::bool(b),
        Wire::Int32(i) => Value::i64(i as i64),
        Wire::Int64(i) => Value::i64(i),
        Wire::Double(f) => Value::f64(f),
        Wire::String(s) => Value::string(s),
        Wire::ObjectId(oid) => Value::string(oid.to_hex()),
        Wire::Binary(bin) => Value::binary(bin.bytes),
        Wire::DateTime(dt) => Value::string(
            dt.try_to_rfc3339_string()
                .unwrap_or_else(|_| dt.to_string()),
        ),
        Wire::Decimal128(d) => Value::string(d.to_string()),
        Wire::Document(document) => {
            let mut out = Vec::with_capacity(document.len());
            for (key, v) in document {
                out.push((key, value(v)?));
            }
            Value::r#struct(out)
        }
        // Empty lists default to float, same as the Arrow path.
        Wire::Array(items) if items.is_empty() => Value::list(Vec::<f32>::new()),
        Wire::Array(items) => {
            if let Some(values) = items
                .iter()
                .map(|b| match b {
                    Wire::Int32(v) => Some(i64::from(*v)),
                    Wire::Int64(v) => Some(*v),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
            {
                return Ok(Value::list(values));
            }
            if let Some(values) = items
                .iter()
                .map(|b| match b {
                    Wire::Int32(v) => Some(f64::from(*v)),
                    Wire::Int64(v) => Some(*v as f64),
                    Wire::Double(v) => Some(*v),
                    Wire::Decimal128(d) => d.to_string().parse().ok(),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
            {
                return Ok(Value::list(values));
            }
            if let Some(values) = items
                .into_iter()
                .map(|b| match b {
                    Wire::String(s) => Some(s),
                    _ => None,
                })
                .collect::<Option<Vec<String>>>()
            {
                return Ok(Value::list(values));
            }
            return Err(Error::InvalidArgument(
                "list mixes types; TopK lists hold numbers or strings, not both".to_string(),
            ));
        }
        other => Value::string(other.to_string()),
    })
}
