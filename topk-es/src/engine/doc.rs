use std::collections::HashMap;

use topk_rs::proto::v1::control::{field_type, field_type_matrix::MatrixValueType, FieldSpec};
use topk_rs::proto::v1::data::{value, Document, Value};

use super::field::IndexKind;
use super::{Schema, RANK_PREFIX};
use crate::api::{Source, SourceFilter, WriteDoc};
use crate::value::ValueExt;
use crate::vector;
use crate::Error;

pub fn decode(source: &SourceFilter, fields: HashMap<String, Value>) -> Source {
    let mut nested = HashMap::new();
    for (key, value) in fields {
        let internal = key.starts_with('_') || key.starts_with(RANK_PREFIX);
        if !internal && source.keep(&key) {
            insert_path(&mut nested, &key, value);
        }
    }

    Source(nested.into())
}

pub fn decode_fields(schema: &Schema, fields: HashMap<String, Value>) -> HashMap<String, Value> {
    let mut flat = HashMap::new();
    for (name, value) in fields {
        flatten_value(schema, &mut flat, name, value);
    }
    flat
}

fn flatten_value(schema: &Schema, out: &mut HashMap<String, Value>, path: String, value: Value) {
    match value.value {
        Some(value::Value::Struct(s)) => {
            for (key, value) in s.fields {
                flatten_value(schema, out, format!("{path}.{key}"), value);
            }
        }
        value => {
            let value = match schema.get(path.as_str()) {
                Some(spec) if is_byte_vector(spec) => Value { value }.into_signed_bytes(),
                Some(spec) if is_timestamp(spec) => Value { value }
                    .as_timestamp()
                    .and_then(crate::date::format_millis)
                    .map(Value::string)
                    .unwrap_or(Value { value: None }),
                _ => Value { value },
            };
            out.insert(path, value);
        }
    }
}

fn is_byte_vector(spec: &FieldSpec) -> bool {
    matches!(
        spec.data_type.as_ref().and_then(|t| t.data_type.as_ref()),
        Some(field_type::DataType::U8Vector(_) | field_type::DataType::BinaryVector(_))
    )
}

fn is_timestamp(spec: &FieldSpec) -> bool {
    matches!(
        spec.data_type.as_ref().and_then(|t| t.data_type.as_ref()),
        Some(field_type::DataType::Timestamp(_))
    )
}

fn insert_path(fields: &mut HashMap<String, Value>, path: &str, value: Value) {
    match path.split_once('.') {
        None => {
            fields.insert(path.to_string(), value);
        }
        Some((head, rest)) => {
            let entry = fields
                .entry(head.to_string())
                .or_insert_with(|| Value::r#struct(HashMap::<String, Value>::new()));
            if let Some(value::Value::Struct(s)) = &mut entry.value {
                insert_path(&mut s.fields, rest, value);
            }
        }
    }
}

pub fn encode_batch(schema: &Schema, docs: Vec<WriteDoc>) -> Result<Vec<Document>, Error> {
    docs.into_iter().map(|doc| encode(schema, doc)).collect()
}

pub fn encode(schema: &Schema, doc: WriteDoc) -> Result<Document, Error> {
    let fields = doc
        .into_fields()
        .into_iter()
        .map(|(name, value)| {
            let value = value.into_inner();
            let spec = schema.get(name.as_str());

            let value = match spec.and_then(|spec| spec.data_type.as_ref()?.data_type.as_ref()) {
                Some(field_type::DataType::F32Vector(_)) => value.to_f32_list().unwrap_or(value),
                Some(field_type::DataType::I8Vector(_)) => value.to_i8_list().unwrap_or(value),
                Some(field_type::DataType::U8Vector(_) | field_type::DataType::BinaryVector(_)) => {
                    value.to_unsigned_bytes().unwrap_or(value)
                }
                Some(field_type::DataType::Matrix(m))
                    if matches!(m.value_type(), MatrixValueType::U8) =>
                {
                    value.to_u8_matrix().unwrap_or(value)
                }
                Some(field_type::DataType::Timestamp(_)) => match value.as_string() {
                    Some(s) => Value::timestamp(crate::date::parse_millis(s)?),
                    None => value,
                },
                _ => value,
            };

            if let Some(IndexKind::Vector(metric)) = spec.map(IndexKind::from) {
                vector::ensure_magnitude(&name, metric, &value)?;
            }

            Ok((name, value))
        })
        .collect::<Result<HashMap<_, _>, Error>>()?;

    Ok(Document { fields })
}
