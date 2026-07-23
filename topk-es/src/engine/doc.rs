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
            insert_path(&mut nested, &key, dejson(value));
        }
    }

    Source(nested.into())
}

// Reverse of `enjson`: a text value that looks like a JSON object/array is a stringified `nested`
// field — parse it back. Dirty by design (a genuine string starting with `{`/`[` would be parsed),
// but scoped to what Kibana's saved objects actually contain.
fn dejson(value: Value) -> Value {
    let parsed = value.as_string().and_then(|s| {
        let s = s.trim_start();
        (s.starts_with('{') || s.starts_with('['))
            .then(|| serde_json::from_str::<serde_json::Value>(s.trim()).ok())
            .flatten()
    });
    match parsed.and_then(|json| Value::try_from(json).ok()) {
        Some(value) => value,
        None => value,
    }
}

// Coerce a value into a text column the way ES does: numbers/bools become their string form, and
// `nested` (object / array-of-objects / empty-array) is serialized to a JSON string. Actual
// strings and scalar arrays (keyword arrays) pass through unchanged.
fn enjson(value: Value) -> Value {
    let json = match serde_json::Value::try_from(value.clone()) {
        Ok(json) => json,
        Err(_) => return value,
    };
    match &json {
        // A scalar string stays a string; everything else — numbers, bools, and any array or
        // object (keyword arrays included, since TopK text columns are scalar) — becomes a JSON
        // string that `dejson` parses back on read.
        serde_json::Value::String(_) => value,
        serde_json::Value::Number(n) => Value::string(n.to_string()),
        serde_json::Value::Bool(b) => Value::string(b.to_string()),
        _ => Value::string(json.to_string()),
    }
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
            let coerced = coerce(schema.get(name.as_str()), name.as_str(), value.into_inner())?;
            Ok((name, coerced))
        })
        .collect::<Result<HashMap<_, _>, Error>>()?;

    Ok(Document { fields })
}

// Coerce each leaf value to its column type, descending into structs (whose sub-specs live inside
// the struct FieldSpec) so nested scalars like `config.buildNum` are matched to their column.
fn coerce(spec: Option<&FieldSpec>, path: &str, value: Value) -> Result<Value, Error> {
    if let Some(value::Value::Struct(s)) = value.value {
        let sub = spec
            .and_then(|sp| sp.data_type.as_ref()?.data_type.as_ref())
            .and_then(|dt| match dt {
                field_type::DataType::Struct(st) => Some(&st.fields),
                _ => None,
            });
        let fields = s
            .fields
            .into_iter()
            .map(|(key, v)| {
                let child_spec = sub.and_then(|m| m.get(&key));
                let child_path = format!("{path}.{key}");
                Ok((key, coerce(child_spec, &child_path, v)?))
            })
            .collect::<Result<_, Error>>()?;
        return Ok(Value {
            value: Some(value::Value::Struct(topk_rs::proto::v1::data::Struct {
                fields,
            })),
        });
    }

    let value = match spec.and_then(|spec| spec.data_type.as_ref()?.data_type.as_ref()) {
        Some(field_type::DataType::F32Vector(_)) => value.to_f32_list().unwrap_or(value),
        Some(field_type::DataType::I8Vector(_)) => value.to_i8_list().unwrap_or(value),
        Some(field_type::DataType::U8Vector(_) | field_type::DataType::BinaryVector(_)) => {
            value.to_unsigned_bytes().unwrap_or(value)
        }
        Some(field_type::DataType::Matrix(m)) if matches!(m.value_type(), MatrixValueType::U8) => {
            value.to_u8_matrix().unwrap_or(value)
        }
        Some(field_type::DataType::Timestamp(_)) => match value.as_string() {
            Some(s) => Value::timestamp(crate::date::parse_millis(s)?),
            None => value,
        },
        // A `nested` field maps to a text column; serialize objects/arrays-of-objects to a JSON
        // string and coerce scalar numbers/bools to strings, matching ES text coercion.
        Some(field_type::DataType::Text(_)) => enjson(value),
        _ => value,
    };

    if let Some(IndexKind::Vector(metric)) = spec.map(IndexKind::from) {
        vector::ensure_magnitude(path, metric, &value)?;
    }

    Ok(value)
}
