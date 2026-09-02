use topk_rs::proto::v1::data::{
    sparse_vector, value::Value as Inner, IntoListValues, IntoMatrixValues, IntoSparseValues, Value,
};

use crate::import::decode::{exact_int, finite, float, floats, int, ints, text};
use crate::import::error::Error;
use crate::import::spec::{Element, Field, Type};

impl Field {
    pub fn coerce(&self, value: Value) -> Result<Value, Error> {
        if value.as_null().is_some() {
            return Ok(value);
        }
        let ty = self.ty;
        // Container types accept JSON in a string cell (CSV, TEXT columns).
        let value = match value.as_string().filter(|_| !ty.is_scalar()) {
            Some(json) => serde_json::from_str::<topk_rs::json::Value>(json)?.into_inner(),
            None => value,
        };
        let cannot = || Error::CannotCoerce(ty);

        Ok(match ty {
            Type::Text => {
                let mut text = text(value)?;
                if let Some(chars) = self.truncate {
                    if let Some((at, _)) = text.char_indices().nth(chars) {
                        text.truncate(at);
                    }
                }
                Value::string(text)
            }
            Type::Int => Value::i64(int(&value).ok_or_else(cannot)?),
            Type::Float => Value::f64(finite(float(&value).ok_or_else(cannot)?)?),
            Type::Bool => Value::bool(
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
                .ok_or_else(cannot)?,
            ),
            // Declared in the schema, carried as an epoch integer. Sources that
            // render a date as text (elasticsearch `_source`, a csv column) parse
            // to the same instant; anything else is a `text` field, not a timestamp.
            Type::Timestamp => Value::i64(
                value
                    .as_string()
                    .and_then(epoch_millis)
                    .or_else(|| int(&value))
                    .ok_or_else(cannot)?,
            ),
            Type::Bytes if value.as_binary().is_some() => value,
            Type::Struct if value.as_struct().is_some() => value,
            Type::Bytes | Type::Struct => return Err(cannot()),
            Type::TextList => Value::list(value.as_string_list().ok_or_else(cannot)?.to_vec()),
            Type::IntList => Value::list(
                ints(&value)
                    .or_else(|| floats(&value)?.into_iter().map(exact_int).collect())
                    .ok_or_else(cannot)?,
            ),
            _ => {
                let (shape, nums) = match ty {
                    Type::FloatList => (Shape::Dense, floats(&value).ok_or_else(cannot)?),
                    _ if ty.is_dense() => (Shape::Dense, dense_floats(value, self)?),
                    _ if ty.is_matrix() => {
                        let cols = self.cols.ok_or_else(|| {
                            Error::InvalidArgument(format!("{ty} requires `cols`"))
                        })?;
                        let nums = match value.value.as_ref() {
                            Some(Inner::Matrix(m)) if m.num_cols == cols => return Ok(value),
                            Some(Inner::Matrix(m)) => {
                                return Err(Error::InvalidArgument(format!(
                                    "matrix has {} columns, declared cols={cols}",
                                    m.num_cols
                                )))
                            }
                            _ => matrix_floats(&value, cols, ty)?,
                        };
                        (Shape::Matrix(cols), nums)
                    }
                    _ => {
                        let (indices, nums) = sparse_pairs(&value, ty)?;
                        (Shape::Sparse(indices), nums)
                    }
                };
                match ty.element().ok_or_else(cannot)? {
                    Element::F32 => typed_value::<f32>(shape, &nums, ty)?,
                    Element::F16 => typed_value::<half::f16>(shape, &nums, ty)?,
                    Element::F8 => typed_value::<float8::F8E4M3>(shape, &nums, ty)?,
                    Element::U8 | Element::Binary => typed_value::<u8>(shape, &nums, ty)?,
                    Element::I8 => typed_value::<i8>(shape, &nums, ty)?,
                }
            }
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

/// Milliseconds since the epoch from a rendered date: RFC 3339 first, then a
/// bare date, which is midnight UTC.
fn epoch_millis(text: &str) -> Option<i64> {
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
}

fn typed_value<T: Numeric>(shape: Shape, nums: &[f64], ty: Type) -> Result<Value, Error>
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
fn dense_floats(value: Value, field: &Field) -> Result<Vec<f64>, Error> {
    let dim = field
        .dim
        .ok_or_else(|| Error::InvalidArgument(format!("{} requires `dim`", field.ty)))?;
    if let Some(bytes) = value.as_binary() {
        return packed_floats(bytes, dim as usize, field.ty);
    }
    let nums = floats(&value).ok_or(Error::CannotCoerce(field.ty))?;
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
fn packed_floats(bytes: &[u8], dim: usize, ty: Type) -> Result<Vec<f64>, Error> {
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
        ty,
        Type::Vector(Element::U8 | Element::I8 | Element::Binary)
    ) {
        if width != 1 {
            return Err(Error::InvalidArgument(format!(
                "{} bytes over dim={dim} is {width} bytes per element, but {ty} reads 1",
                bytes.len()
            )));
        }
        return Ok(match ty {
            Type::Vector(Element::I8) => bytes.iter().map(|&b| b as i8 as f64).collect(),
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

/// A flat numeric list as a matrix `cols` wide; multi-vector sources flatten
/// their rows (colbert: one `FLOAT[]` per document). Not for binary: without a
/// `dim` the element width is ambiguous (16 KiB over cols=128 is 32 f32 rows
/// or 64 f16 rows).
fn matrix_floats(value: &Value, cols: u32, ty: Type) -> Result<Vec<f64>, Error> {
    let nums = floats(value).ok_or(Error::CannotCoerce(ty))?;
    if cols == 0 || nums.len() % cols as usize != 0 {
        return Err(Error::InvalidArgument(format!(
            "{} values do not divide into cols={cols} (a flat list becomes a matrix \
             cols wide; rows follow from the length)",
            nums.len()
        )));
    }
    Ok(nums)
}

/// (indices, values) sorted by index, from topk's sparse form, a struct of
/// parallel `indices` and `values` lists, or a struct of numeric keys.
fn sparse_pairs(value: &Value, ty: Type) -> Result<(Vec<u32>, Vec<f64>), Error> {
    let mut pairs: Vec<(u32, f64)> = match value.value.as_ref() {
        Some(Inner::SparseVector(sparse)) => match sparse.values.as_ref() {
            Some(sparse_vector::Values::F32(f)) => sparse
                .indices
                .iter()
                .copied()
                .zip(f.values.iter().map(|&v| v as f64))
                .collect(),
            _ => return Err(Error::CannotCoerce(ty)),
        },
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
