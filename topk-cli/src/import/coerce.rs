use topk_rs::proto::v1::data::{
    image, list, matrix, sparse_vector, value::Value as Inner, IntoListValues, IntoMatrixValues,
    IntoSparseValues, Value,
};

use crate::import::error::{Error, MAX_IMAGE_BYTES};
use crate::import::spec::Field;
use topk_rs::proto::v1::control::{
    field_type::DataType, field_type_list::ListValueType, field_type_matrix::MatrixValueType,
};

impl Field {
    pub fn coerce(&self, value: Value) -> Result<Value, Error> {
        let ty = self
            .schema
            .data_type
            .as_ref()
            .and_then(|ty| ty.data_type.as_ref())
            .ok_or_else(|| Error::InvalidArgument("field has no data type".into()))?;
        if value.as_null().is_some() {
            return Ok(value);
        }
        // Container types accept JSON in a string cell (CSV, TEXT columns).
        let value = match value.as_string().filter(|_| {
            !matches!(
                ty,
                DataType::Text(_)
                    | DataType::Integer(_)
                    | DataType::Float(_)
                    | DataType::Boolean(_)
                    | DataType::Image(_)
                    | DataType::Bytes(_)
                    | DataType::Timestamp(_)
            )
        }) {
            Some(json) => serde_json::from_str::<topk_rs::json::Value>(json)?.into_inner(),
            None => value,
        };

        Ok(match ty {
            DataType::Text(_) => {
                let mut text = text(value)?;
                if let Some(chars) = self.truncate {
                    if let Some((at, _)) = text.char_indices().nth(chars) {
                        text.truncate(at);
                    }
                }
                Value::string(text)
            }
            DataType::Integer(_) => Value::i64(int(&value).ok_or(Error::CannotCoerce("int"))?),
            DataType::Float(_) => {
                Value::f64(finite(float(&value).ok_or(Error::CannotCoerce("float"))?)?)
            }
            DataType::Boolean(_) => Value::bool(
                match value.value.as_ref() {
                    Some(Inner::Bool(b)) => Some(*b),
                    Some(Inner::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
                        "true" | "t" | "1" | "yes" | "y" => Some(true),
                        "false" | "f" | "0" | "no" | "n" => Some(false),
                        _ => None,
                    },
                    Some(Inner::I32(_) | Inner::I64(_) | Inner::U32(_) | Inner::U64(_)) => {
                        int(&value).map(|n| n != 0)
                    }
                    _ => None,
                }
                .ok_or(Error::CannotCoerce("bool"))?,
            ),
            // Declared in the schema, carried as an epoch integer. Sources that
            // render a date as text (elasticsearch `_source`, a csv column) parse
            // to the same instant; anything else is a `text` field, not a timestamp.
            DataType::Timestamp(_) => Value::i64(
                value
                    .as_string()
                    .and_then(|text| {
                        // Milliseconds since the epoch from a rendered date: RFC 3339 first, then a
                        // bare date, which is midnight UTC.
                        let text = text.trim();
                        if let Ok(stamp) = chrono::DateTime::parse_from_rfc3339(text) {
                            return Some(stamp.timestamp_millis());
                        }
                        let date = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()?;
                        Some(
                            date.and_time(chrono::NaiveTime::MIN)
                                .and_utc()
                                .timestamp_millis(),
                        )
                    })
                    .or_else(|| int(&value))
                    .ok_or(Error::CannotCoerce("timestamp"))?,
            ),
            DataType::Image(_) => {
                let value = match value.as_struct().and_then(|s| s.get("bytes")) {
                    Some(bytes) => bytes.clone(),
                    None => value,
                };
                let bytes = match value.value {
                    Some(Inner::Image(image)) => image
                        .data
                        .ok_or(Error::CannotCoerce("image"))?
                        .bytes()
                        .clone(),
                    Some(Inner::Binary(bytes)) => bytes,
                    Some(Inner::String(uri)) => image::Data::try_from(uri)?.bytes().clone(),
                    _ => return Err(Error::CannotCoerce("image")),
                };
                let data = image::Data::from_bytes(bytes).ok_or_else(|| {
                    Error::InvalidArgument("image bytes are not a JPEG or PNG".into())
                })?;
                if data.bytes().len() > MAX_IMAGE_BYTES {
                    return Err(Error::InvalidArgument(format!(
                        "image is {} bytes, over the {MAX_IMAGE_BYTES} byte limit",
                        data.bytes().len()
                    )));
                }
                Value::image(data)
            }
            DataType::Bytes(_) if value.as_binary().is_some() => value,
            DataType::Struct(_) if value.as_struct().is_some() => value,
            DataType::Bytes(_) => return Err(Error::CannotCoerce("bytes")),
            DataType::Struct(_) => return Err(Error::CannotCoerce("struct")),
            DataType::List(list) if list.value_type() == ListValueType::String => Value::list(
                value
                    .as_string_list()
                    .ok_or(Error::CannotCoerce("text_list"))?
                    .to_vec(),
            ),
            DataType::List(list) if list.value_type() == ListValueType::Integer => Value::list(
                ints(&value)
                    .or_else(|| floats(&value)?.into_iter().map(exact_int).collect())
                    .ok_or(Error::CannotCoerce("int_list"))?,
            ),
            DataType::List(list) if list.value_type() == ListValueType::Float => {
                typed_value::<f32>(
                    Shape::Dense,
                    &floats(&value).ok_or(Error::CannotCoerce("float_list"))?,
                    "float_list",
                )?
            }
            DataType::F32Vector(v) => typed_value::<f32>(
                Shape::Dense,
                &dense_floats(value, v.dimension, ty, "f32_vector")?,
                "f32_vector",
            )?,
            DataType::F16Vector(v) => typed_value::<half::f16>(
                Shape::Dense,
                &dense_floats(value, v.dimension, ty, "f16_vector")?,
                "f16_vector",
            )?,
            DataType::F8Vector(v) => typed_value::<float8::F8E4M3>(
                Shape::Dense,
                &dense_floats(value, v.dimension, ty, "f8_vector")?,
                "f8_vector",
            )?,
            DataType::U8Vector(v) => typed_value::<u8>(
                Shape::Dense,
                &dense_floats(value, v.dimension, ty, "u8_vector")?,
                "u8_vector",
            )?,
            DataType::I8Vector(v) => typed_value::<i8>(
                Shape::Dense,
                &dense_floats(value, v.dimension, ty, "i8_vector")?,
                "i8_vector",
            )?,
            DataType::BinaryVector(v) => typed_value::<u8>(
                Shape::Dense,
                &dense_floats(value, v.dimension, ty, "binary_vector")?,
                "binary_vector",
            )?,
            DataType::F32SparseVector(_) => {
                let (indices, nums) = sparse_pairs(&value, "f32_sparse_vector")?;
                typed_value::<f32>(Shape::Sparse(indices), &nums, "f32_sparse_vector")?
            }
            DataType::F16SparseVector(_) => {
                let (indices, nums) = sparse_pairs(&value, "f16_sparse_vector")?;
                typed_value::<half::f16>(Shape::Sparse(indices), &nums, "f16_sparse_vector")?
            }
            DataType::F8SparseVector(_) => {
                let (indices, nums) = sparse_pairs(&value, "f8_sparse_vector")?;
                typed_value::<float8::F8E4M3>(Shape::Sparse(indices), &nums, "f8_sparse_vector")?
            }
            DataType::U8SparseVector(_) => {
                let (indices, nums) = sparse_pairs(&value, "u8_sparse_vector")?;
                typed_value::<u8>(Shape::Sparse(indices), &nums, "u8_sparse_vector")?
            }
            DataType::I8SparseVector(_) => {
                let (indices, nums) = sparse_pairs(&value, "i8_sparse_vector")?;
                typed_value::<i8>(Shape::Sparse(indices), &nums, "i8_sparse_vector")?
            }
            DataType::Matrix(matrix) => {
                let cols = matrix.dimension;
                let nums = match value.value.as_ref() {
                    Some(Inner::Matrix(m)) if m.num_cols == cols => match m.values.as_ref() {
                        Some(matrix::Values::F32(v)) => {
                            v.values.iter().map(|&n| n as f64).collect()
                        }
                        Some(matrix::Values::F16(v)) => {
                            v.as_ref().iter().map(|n| n.to_f64()).collect()
                        }
                        Some(matrix::Values::F8(v)) => {
                            v.as_ref().iter().map(|n| n.to_f64()).collect()
                        }
                        Some(matrix::Values::U8(v)) => v.values.iter().map(|&n| n as f64).collect(),
                        Some(matrix::Values::I8(v)) => {
                            v.as_ref().iter().map(|&n| n as f64).collect()
                        }
                        None => return Err(Error::CannotCoerce("matrix")),
                    },
                    Some(Inner::Matrix(m)) => {
                        return Err(Error::InvalidArgument(format!(
                            "matrix has {} columns, declared cols={cols}",
                            m.num_cols
                        )))
                    }
                    _ => {
                        // A flat numeric list as a matrix `cols` wide; multi-vector sources flatten
                        // their rows (colbert: one `FLOAT[]` per document). Not for binary: without a
                        // `dim` the element width is ambiguous (16 KiB over cols=128 is 32 f32 rows
                        // or 64 f16 rows).
                        let nums = floats(&value).ok_or(Error::CannotCoerce("matrix"))?;
                        if cols == 0 || nums.len() % cols as usize != 0 {
                            return Err(Error::InvalidArgument(format!(
                                "{} values do not divide into cols={cols} (a flat list becomes a matrix \
                                 cols wide; rows follow from the length)",
                                nums.len()
                            )));
                        }
                        nums
                    }
                };
                match matrix.value_type() {
                    MatrixValueType::F32 => {
                        typed_value::<f32>(Shape::Matrix(cols), &nums, "f32_matrix")?
                    }
                    MatrixValueType::F16 => {
                        typed_value::<half::f16>(Shape::Matrix(cols), &nums, "f16_matrix")?
                    }
                    MatrixValueType::F8 => {
                        typed_value::<float8::F8E4M3>(Shape::Matrix(cols), &nums, "f8_matrix")?
                    }
                    MatrixValueType::U8 => {
                        typed_value::<u8>(Shape::Matrix(cols), &nums, "u8_matrix")?
                    }
                    MatrixValueType::I8 => {
                        typed_value::<i8>(Shape::Matrix(cols), &nums, "i8_matrix")?
                    }
                    _ => {
                        return Err(Error::InvalidArgument(
                            "unsupported matrix element type".into(),
                        ))
                    }
                }
            }
            _ => return Err(Error::InvalidArgument("unsupported field type".into())),
        })
    }
}

enum Shape {
    Dense,
    Matrix(u32),
    Sparse(Vec<u32>),
}

/// One element of a typed vector, matrix or sparse vector from the f64 a
/// source produced. Floats narrow; integers must be exact.
trait Numeric: Sized
where
    Vec<Self>: IntoListValues + IntoMatrixValues + IntoSparseValues,
{
    fn from_f64(n: f64) -> Option<Self>;
}

impl Numeric for f32 {
    fn from_f64(n: f64) -> Option<f32> {
        Some(n as f32).filter(|v| v.is_finite())
    }
}

impl Numeric for half::f16 {
    fn from_f64(n: f64) -> Option<half::f16> {
        Some(half::f16::from_f64(n)).filter(|v| v.is_finite())
    }
}

impl Numeric for float8::F8E4M3 {
    fn from_f64(n: f64) -> Option<float8::F8E4M3> {
        Some(float8::F8E4M3::from_f64(n)).filter(|v| v.is_finite())
    }
}

impl Numeric for u8 {
    fn from_f64(n: f64) -> Option<u8> {
        u8::try_from(exact_int(n)?).ok()
    }
}

impl Numeric for i8 {
    fn from_f64(n: f64) -> Option<i8> {
        i8::try_from(exact_int(n)?).ok()
    }
}

fn typed_value<T: Numeric>(shape: Shape, nums: &[f64], ty: &'static str) -> Result<Value, Error>
where
    Vec<T>: IntoListValues + IntoMatrixValues + IntoSparseValues,
{
    let values: Vec<T> = nums
        .iter()
        .map(|&n| T::from_f64(finite(n)?).ok_or(Error::CannotCoerce(ty)))
        .collect::<Result<_, _>>()?;
    Ok(match shape {
        Shape::Dense => Value::list(values),
        Shape::Matrix(cols) => Value::matrix(cols, values),
        Shape::Sparse(indices) => Value::sparse_vector(indices, values),
    })
}

/// Numeric elements of a declared vector, checked against `dim`. Accepts a JSON
/// string cell (pgvector's text form) and a binary cell holding a packed array.
fn dense_floats(
    value: Value,
    dim: u32,
    schema: &DataType,
    ty: &'static str,
) -> Result<Vec<f64>, Error> {
    if let Some(bytes) = value.as_binary() {
        return packed_floats(bytes, dim as usize, schema, ty);
    }
    let nums = floats(&value).ok_or(Error::CannotCoerce(ty))?;
    if nums.len() != dim as usize {
        return Err(Error::InvalidArgument(format!(
            "vector has {} values, declared dim={dim}",
            nums.len()
        )));
    }
    Ok(nums)
}

/// Elements of a binary cell holding a packed little-endian array. The element
/// width is `len / dim`, not the declared type's: blobs in the wild routinely
/// disagree with the target (numpy writes f64, our own datasets f16).
fn packed_floats(
    bytes: &[u8],
    dim: usize,
    schema: &DataType,
    ty: &'static str,
) -> Result<Vec<f64>, Error> {
    if dim == 0 || bytes.len() % dim != 0 {
        // The dims this byte length could mean, for the error that says `dim` is wrong.
        let dims: Vec<String> = [(2, "f16"), (4, "f32"), (8, "f64")]
            .iter()
            .filter(|(width, _)| bytes.len() % width == 0)
            .map(|(width, name)| format!("{} as {name}", bytes.len() / width))
            .collect();
        return Err(Error::InvalidArgument(format!(
            "{} bytes does not divide into dim={dim} (a binary cell decodes as a packed \
             little-endian array{})",
            bytes.len(),
            match dims.is_empty() {
                true => String::new(),
                false => format!("; this length is dim {}", dims.join(", ")),
            }
        )));
    }
    let width = bytes.len() / dim;
    // Byte vectors read one byte per element; a wider cell would be lossy.
    if matches!(
        schema,
        DataType::U8Vector(_) | DataType::I8Vector(_) | DataType::BinaryVector(_)
    ) {
        if width != 1 {
            return Err(Error::InvalidArgument(format!(
                "{} bytes over dim={dim} is {width} bytes per element, but {ty} reads 1",
                bytes.len()
            )));
        }
        return Ok(match schema {
            DataType::I8Vector(_) => bytes.iter().map(|&b| b as i8 as f64).collect(),
            _ => bytes.iter().map(|&b| f64::from(b)).collect(),
        });
    }
    // `chunks_exact` guarantees each chunk's length, so `try_into` cannot fail.
    Ok(match width {
        2 => bytes
            .chunks_exact(2)
            .map(|c| f64::from(half::f16::from_le_bytes(c.try_into().unwrap())))
            .collect(),
        4 => bytes
            .chunks_exact(4)
            .map(|c| f64::from(f32::from_le_bytes(c.try_into().unwrap())))
            .collect(),
        8 => bytes
            .chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect(),
        _ => {
            return Err(Error::InvalidArgument(format!(
                "{} bytes over dim={dim} is {width} bytes per element; a packed binary cell \
                 holds 2 (f16), 4 (f32) or 8 (f64) bytes per element",
                bytes.len()
            )))
        }
    })
}

/// (indices, values) sorted by index, from topk's sparse form, a struct of
/// parallel `indices` and `values` lists, or a struct of numeric keys.
fn sparse_pairs(value: &Value, ty: &'static str) -> Result<(Vec<u32>, Vec<f64>), Error> {
    let mut pairs: Vec<(u32, f64)> = match value.value.as_ref() {
        Some(Inner::SparseVector(sparse)) => {
            let values: Vec<f64> = match sparse.values.as_ref() {
                Some(sparse_vector::Values::F32(v)) => v.values.iter().map(|&n| n as f64).collect(),
                Some(sparse_vector::Values::F16(v)) => {
                    v.as_ref().iter().map(|n| n.to_f64()).collect()
                }
                Some(sparse_vector::Values::F8(v)) => {
                    v.as_ref().iter().map(|n| n.to_f64()).collect()
                }
                Some(sparse_vector::Values::U8(v)) => v.values.iter().map(|&n| n as f64).collect(),
                Some(sparse_vector::Values::I8(v)) => {
                    v.as_ref().iter().map(|&n| n as f64).collect()
                }
                None => return Err(Error::CannotCoerce(ty)),
            };
            if sparse.indices.len() != values.len() {
                return Err(Error::CannotCoerce(ty));
            }
            sparse.indices.iter().copied().zip(values).collect()
        }
        _ => {
            let entries = value.as_struct().ok_or(Error::CannotCoerce(ty))?;
            match (entries.get("indices"), entries.get("values")) {
                (Some(indices), Some(values)) => {
                    let indices = ints(indices).ok_or(Error::CannotCoerce(ty))?;
                    let values = floats(values).ok_or(Error::CannotCoerce(ty))?;
                    if indices.len() != values.len() {
                        return Err(Error::InvalidArgument(format!(
                            "sparse vector has {} indices and {} values",
                            indices.len(),
                            values.len()
                        )));
                    }
                    let mut pairs = Vec::with_capacity(indices.len());
                    for (index, value) in indices.into_iter().zip(values) {
                        let index = u32::try_from(index).map_err(|_| Error::CannotCoerce(ty))?;
                        pairs.push((index, value));
                    }
                    pairs
                }
                _ => {
                    let mut pairs = Vec::with_capacity(entries.len());
                    for (key, entry) in entries {
                        // duckdb unifies jsonl schemas by filling absent keys with null.
                        if entry.as_null().is_some() {
                            continue;
                        }
                        let index: u32 = key.trim().parse().map_err(|_| Error::CannotCoerce(ty))?;
                        pairs.push((index, float(entry).ok_or(Error::CannotCoerce(ty))?));
                    }
                    pairs
                }
            }
        }
    };
    pairs.sort_by_key(|(index, _)| *index);
    Ok(pairs.into_iter().unzip())
}

pub fn text(value: Value) -> Result<String, Error> {
    Ok(match value.value {
        Some(Inner::String(s)) => s,
        Some(Inner::Binary(bytes)) => String::from_utf8(bytes.into()).map_err(|_| {
            Error::InvalidArgument(
                "declared as text but the bytes are not valid UTF-8; declare `bytes` to \
                 keep them as binary"
                    .to_string(),
            )
        })?,
        Some(Inner::Bool(b)) => b.to_string(),
        Some(Inner::I32(n)) => n.to_string(),
        Some(Inner::I64(n)) => n.to_string(),
        Some(Inner::U32(n)) => n.to_string(),
        Some(Inner::U64(n)) => n.to_string(),
        Some(Inner::F32(f)) => f.to_string(),
        Some(Inner::F64(f)) => f.to_string(),
        value => serde_json::Value::try_from(Value { value })?.to_string(),
    })
}

/// An exact i64 from whatever numeric shape a source produced.
fn int(value: &Value) -> Option<i64> {
    match value.value.as_ref()? {
        Inner::I32(n) => Some(*n as i64),
        Inner::I64(n) => Some(*n),
        Inner::U32(n) => Some(*n as i64),
        Inner::U64(n) => i64::try_from(*n).ok(),
        Inner::F32(f) => exact_int(*f as f64),
        Inner::F64(f) => exact_int(*f),
        Inner::Bool(b) => Some(*b as i64),
        Inner::String(s) => parse_int(s),
        _ => None,
    }
}

fn float(value: &Value) -> Option<f64> {
    match value.value.as_ref()? {
        Inner::I32(n) => Some(*n as f64),
        Inner::I64(n) => Some(*n as f64),
        Inner::U32(n) => Some(*n as f64),
        Inner::U64(n) => Some(*n as f64),
        Inner::F32(f) => Some(*f as f64),
        Inner::F64(f) => Some(*f),
        Inner::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Exact i64 from a float, by round-trip: rejects fractions, NaN/inf, and
/// out-of-range.
fn exact_int(f: f64) -> Option<i64> {
    ((i64::MIN as f64..-(i64::MIN as f64)).contains(&f) && f as i64 as f64 == f).then_some(f as i64)
}

/// Exact i64 from a rendered number; an all-zero fraction is dropped (" 3.00 ").
/// Parsed as an integer, not through f64, so ids past 2^53 stay exact.
fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim();
    match s.split_once('.') {
        None => s.parse().ok(),
        Some((int, frac)) if !frac.is_empty() && frac.bytes().all(|b| b == b'0') => {
            int.parse().ok()
        }
        Some(_) => None,
    }
}

fn ints(value: &Value) -> Option<Vec<i64>> {
    value
        .as_i64_list()
        .map(|v| v.to_vec())
        .or_else(|| {
            value
                .as_i32_list()
                .map(|v| v.iter().map(|&n| n as i64).collect())
        })
        .or_else(|| {
            value
                .as_u32_list()
                .map(|v| v.iter().map(|&n| n as i64).collect())
        })
        .or_else(|| {
            value
                .as_u64_list()
                .and_then(|v| v.iter().map(|&n| i64::try_from(n).ok()).collect())
        })
        .or_else(|| {
            value
                .as_u8_list()
                .map(|v| v.iter().map(|&n| n as i64).collect())
        })
        .or_else(|| {
            value
                .as_i8_list()
                .map(|v| v.iter().map(|&n| n as i64).collect())
        })
        .or_else(|| {
            value
                .as_string_list()
                .and_then(|v| v.iter().map(|s| parse_int(s)).collect())
        })
}

pub fn floats(value: &Value) -> Option<Vec<f64>> {
    if let Some(Inner::List(list)) = value.value.as_ref() {
        match list.values.as_ref()? {
            list::Values::F16(v) => return Some(v.as_ref().iter().map(|n| n.to_f64()).collect()),
            list::Values::F8(v) => return Some(v.as_ref().iter().map(|n| n.to_f64()).collect()),
            _ => {}
        }
    }
    value
        .as_f64_list()
        .map(|v| v.to_vec())
        .or_else(|| {
            value
                .as_f32_list()
                .map(|v| v.iter().map(|&n| n as f64).collect())
        })
        .or_else(|| {
            value
                .as_u64_list()
                .map(|v| v.iter().map(|&n| n as f64).collect())
        })
        .or_else(|| ints(value).map(|v| v.iter().map(|&n| n as f64).collect()))
        .or_else(|| {
            value
                .as_string_list()
                .and_then(|v| v.iter().map(|s| s.trim().parse().ok()).collect())
        })
}

fn finite(f: f64) -> Result<f64, Error> {
    match f.is_finite() {
        true => Ok(f),
        false => Err(Error::InvalidArgument(format!("non-finite float {f}"))),
    }
}
