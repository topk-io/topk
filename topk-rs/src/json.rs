use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

use serde::de::{Error as DeError, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::ser::{Error as SerError, SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::proto::v1::data::{list, matrix, sparse_vector, value, vector, Value as TopkValue};

// A JSON array maps onto a homogeneous TopK list or matrix, so these rejections
// are shared by every path that walks one.
const MIXED_ARRAY: &str = "JSON arrays must contain only numbers or strings";
const INVALID_ARRAY: &str = "JSON arrays must contain only numbers, strings, or numeric arrays";
const NUMBER_RANGE: &str = "JSON number is outside TopK's supported numeric range";

/// A [`TopkValue`] that serializes to, and deserializes from, JSON directly —
/// no intermediate JSON document is built in either direction.
#[derive(Clone, Debug, PartialEq)]
pub struct Value(pub TopkValue);

impl<T: Into<TopkValue>> From<T> for Value {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Value {
    pub fn into_inner(self) -> TopkValue {
        self.0
    }
}

impl Deref for Value {
    type Target = TopkValue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer
            .deserialize_any(ValueVisitor)?
            .map(Self)
            .map_err(DeError::custom)
    }
}

/// A [`Value`] whose *shape* errors — a mixed array, a numeric struct field
/// name, ... — are captured instead of raised, so a caller that reports errors
/// per value (a bulk write, say) can keep reading the surrounding document.
/// Malformed JSON still fails the deserializer.
pub struct LenientValue(Parsed);

impl LenientValue {
    pub fn into_result(self) -> Result<TopkValue, crate::Error> {
        self.0
    }
}

impl<'de> Deserialize<'de> for LenientValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor).map(Self)
    }
}

// Deserialization
//
// Every visitor below yields `Parsed`: JSON that parsed cleanly but does not
// describe a TopK value comes back as `Err`, keeping the two failure kinds —
// malformed JSON and unrepresentable value — apart.
type Parsed = Result<TopkValue, crate::Error>;

fn invalid(reason: &str) -> crate::Error {
    crate::Error::InvalidArgument(reason.into())
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Parsed;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_unit<E: DeError>(self) -> Result<Parsed, E> {
        Ok(Ok(TopkValue::null()))
    }

    fn visit_none<E: DeError>(self) -> Result<Parsed, E> {
        Ok(Ok(TopkValue::null()))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Parsed, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Parsed, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_bool<E: DeError>(self, value: bool) -> Result<Parsed, E> {
        Ok(Ok(TopkValue::bool(value)))
    }

    fn visit_i64<E: DeError>(self, value: i64) -> Result<Parsed, E> {
        Ok(Ok(TopkValue::i64(value)))
    }

    fn visit_u64<E: DeError>(self, value: u64) -> Result<Parsed, E> {
        Ok(Ok(from_u64(value)))
    }

    fn visit_i128<E: DeError>(self, _: i128) -> Result<Parsed, E> {
        Ok(Err(invalid(NUMBER_RANGE)))
    }

    fn visit_u128<E: DeError>(self, _: u128) -> Result<Parsed, E> {
        Ok(Err(invalid(NUMBER_RANGE)))
    }

    fn visit_f64<E: DeError>(self, value: f64) -> Result<Parsed, E> {
        Ok(Ok(TopkValue::f64(value)))
    }

    fn visit_str<E: DeError>(self, value: &str) -> Result<Parsed, E> {
        Ok(Ok(TopkValue::string(value)))
    }

    fn visit_string<E: DeError>(self, value: String) -> Result<Parsed, E> {
        Ok(Ok(TopkValue::string(value)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Parsed, A::Error> {
        let mut items = Items::Empty;

        while let Some(item) = seq.next_element::<Item>()? {
            items = match items.push(item) {
                Ok(items) => items,
                Err(e) => {
                    while seq.next_element::<IgnoredAny>()?.is_some() {}
                    return Ok(Err(e));
                }
            };
        }

        Ok(items.finish())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Parsed, A::Error> {
        let Some(key) = map.next_key::<Key>()? else {
            return Ok(Ok(TopkValue::r#struct(HashMap::<String, TopkValue>::new())));
        };

        // An object whose every key is a `u32` index and every value a number is
        // a sparse vector; anything else is a struct.
        match key {
            Key::Index(index) => {
                let mut indices = Vec::with_capacity(map.size_hint().unwrap_or(0) + 1);
                let mut values = Vec::with_capacity(indices.capacity());

                let mut key = Some(index);
                while let Some(index) = key {
                    let Some(value) = map.next_value::<Number>()?.0 else {
                        return drain(map).map(|()| {
                            Err(invalid(
                                "JSON objects with numeric keys are sparse vectors \
                                 and must have numeric values",
                            ))
                        });
                    };
                    indices.push(index);
                    values.push(value);

                    key = match map.next_key::<Key>()? {
                        Some(Key::Index(index)) => Some(index),
                        Some(Key::Name(_)) => {
                            return drain_entry(map).map(|()| {
                                Err(invalid(
                                    "JSON objects must not mix numeric indices with field names",
                                ))
                            })
                        }
                        None => None,
                    };
                }

                Ok(Ok(TopkValue::f32_sparse_vector(indices, values)))
            }
            Key::Name(name) => {
                let mut fields = HashMap::with_capacity(map.size_hint().unwrap_or(0) + 1);

                let mut key = Some(name);
                while let Some(name) = key {
                    match map.next_value::<LenientValue>()?.0 {
                        Ok(value) => fields.insert(name, value),
                        Err(e) => return drain(map).map(|()| Err(e)),
                    };

                    key = match map.next_key::<Key>()? {
                        Some(Key::Name(name)) => Some(name),
                        Some(Key::Index(_)) => {
                            return drain_entry(map).map(|()| {
                                Err(invalid("Struct field names must not be numeric indices"))
                            })
                        }
                        None => None,
                    };
                }

                Ok(Ok(TopkValue::r#struct(fields)))
            }
        }
    }
}

fn from_u64(value: u64) -> TopkValue {
    match i64::try_from(value) {
        Ok(value) => TopkValue::i64(value),
        Err(_) => TopkValue::u64(value),
    }
}

// A visitor must consume its whole map even once it has given up, or the
// format's parser is left standing mid-object.
fn drain<'de, A: MapAccess<'de>>(mut map: A) -> Result<(), A::Error> {
    while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
    Ok(())
}

// Same, for a key whose value has not been read yet.
fn drain_entry<'de, A: MapAccess<'de>>(mut map: A) -> Result<(), A::Error> {
    map.next_value::<IgnoredAny>()?;
    drain(map)
}

/// The element type a JSON array has settled on. Numbers accumulate into the
/// narrowest integer width that holds every value seen so far and widen — never
/// narrow — as the array is read, so callers that later match against a
/// collection schema (e.g. topk-es `engine::doc`) still see the original values.
enum Items {
    Empty,
    I64(Vec<i64>),
    U64(Vec<u64>),
    F32(Vec<f32>),
    Str(Vec<String>),
    Matrix { num_cols: usize, values: Vec<f32> },
}

impl Items {
    fn push(self, item: Item) -> Result<Self, crate::Error> {
        Ok(match (self, item) {
            (_, Item::Invalid(reason)) => return Err(invalid(reason)),

            // The first element fixes the array's kind.
            (Items::Empty, Item::I64(v)) => Items::I64(vec![v]),
            (Items::Empty, Item::U64(v)) => Items::U64(vec![v]),
            (Items::Empty, Item::F64(v)) => Items::F32(vec![v as f32]),
            (Items::Empty, Item::Str(v)) => Items::Str(vec![v]),
            (Items::Empty, Item::Row(row)) => match row.is_empty() {
                true => return Err(invalid("JSON matrices must have at least one column")),
                false => Items::Matrix {
                    num_cols: row.len(),
                    values: row,
                },
            },

            (Items::I64(mut values), Item::I64(v)) => {
                values.push(v);
                Items::I64(values)
            }
            (Items::I64(values), Item::U64(v)) => match values.iter().all(|v| *v >= 0) {
                true => {
                    let mut values: Vec<u64> = values.iter().map(|v| *v as u64).collect();
                    values.push(v);
                    Items::U64(values)
                }
                // No integer width holds both a negative value and one past
                // `i64::MAX`, so the array becomes floating point.
                false => Items::F32(widen(values, v as f32)),
            },
            (Items::I64(values), Item::F64(v)) => Items::F32(widen(values, v as f32)),

            (Items::U64(mut values), Item::U64(v)) => {
                values.push(v);
                Items::U64(values)
            }
            (Items::U64(values), Item::I64(v)) => match u64::try_from(v) {
                Ok(v) => {
                    let mut values = values;
                    values.push(v);
                    Items::U64(values)
                }
                Err(_) => Items::F32(widen(values, v as f32)),
            },
            (Items::U64(values), Item::F64(v)) => Items::F32(widen(values, v as f32)),

            (Items::F32(mut values), Item::I64(v)) => {
                values.push(v as f32);
                Items::F32(values)
            }
            (Items::F32(mut values), Item::U64(v)) => {
                values.push(v as f32);
                Items::F32(values)
            }
            (Items::F32(mut values), Item::F64(v)) => {
                values.push(v as f32);
                Items::F32(values)
            }

            (Items::Str(mut values), Item::Str(v)) => {
                values.push(v);
                Items::Str(values)
            }

            (
                Items::Matrix {
                    num_cols,
                    mut values,
                },
                Item::Row(row),
            ) => {
                if row.len() != num_cols {
                    return Err(invalid("JSON matrix rows must have the same length"));
                }
                values.extend_from_slice(&row);
                Items::Matrix { num_cols, values }
            }

            (Items::Matrix { .. }, _) => return Err(invalid(INVALID_ARRAY)),
            _ => return Err(invalid(MIXED_ARRAY)),
        })
    }

    fn finish(self) -> Parsed {
        Ok(match self {
            // An empty array has no element type; f32 matches the JS/Python SDKs.
            Items::Empty => TopkValue::list(Vec::<f32>::new()),
            Items::I64(values) => TopkValue::list(values),
            Items::U64(values) => TopkValue::list(values),
            Items::F32(values) => TopkValue::list(values),
            Items::Str(values) => TopkValue::list(values),
            Items::Matrix { num_cols, values } => {
                if values.len() >= u32::MAX as usize {
                    return Err(invalid("JSON matrix has too many values"));
                }
                TopkValue::matrix(num_cols as u32, values)
            }
        })
    }
}

fn widen<T: Copy + IntoF32>(values: Vec<T>, extra: f32) -> Vec<f32> {
    let mut widened = Vec::with_capacity(values.len() + 1);
    widened.extend(values.iter().map(|v| v.into_f32()));
    widened.push(extra);
    widened
}

// `as` casts, not `Into`: `i64`/`u64` lose precision on the way to `f32`.
trait IntoF32 {
    fn into_f32(self) -> f32;
}

impl IntoF32 for i64 {
    fn into_f32(self) -> f32 {
        self as f32
    }
}

impl IntoF32 for u64 {
    fn into_f32(self) -> f32 {
        self as f32
    }
}

/// One element of a JSON array, read without materializing a JSON value.
enum Item {
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
    Row(Vec<f32>),
    Invalid(&'static str),
}

impl<'de> Deserialize<'de> for Item {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ItemVisitor)
    }
}

struct ItemVisitor;

impl<'de> Visitor<'de> for ItemVisitor {
    type Value = Item;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a number, a string, or an array of numbers")
    }

    fn visit_bool<E: DeError>(self, _: bool) -> Result<Item, E> {
        Ok(Item::Invalid(INVALID_ARRAY))
    }

    fn visit_unit<E: DeError>(self) -> Result<Item, E> {
        Ok(Item::Invalid(INVALID_ARRAY))
    }

    fn visit_none<E: DeError>(self) -> Result<Item, E> {
        Ok(Item::Invalid(INVALID_ARRAY))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Item, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(self, deserializer: D) -> Result<Item, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_i64<E: DeError>(self, value: i64) -> Result<Item, E> {
        Ok(Item::I64(value))
    }

    fn visit_u64<E: DeError>(self, value: u64) -> Result<Item, E> {
        Ok(match i64::try_from(value) {
            Ok(value) => Item::I64(value),
            Err(_) => Item::U64(value),
        })
    }

    fn visit_i128<E: DeError>(self, _: i128) -> Result<Item, E> {
        Ok(Item::Invalid(NUMBER_RANGE))
    }

    fn visit_u128<E: DeError>(self, _: u128) -> Result<Item, E> {
        Ok(Item::Invalid(NUMBER_RANGE))
    }

    fn visit_f64<E: DeError>(self, value: f64) -> Result<Item, E> {
        Ok(Item::F64(value))
    }

    fn visit_str<E: DeError>(self, value: &str) -> Result<Item, E> {
        Ok(Item::Str(value.to_string()))
    }

    fn visit_string<E: DeError>(self, value: String) -> Result<Item, E> {
        Ok(Item::Str(value))
    }

    // A nested array is a matrix row, and matrices are numeric.
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Item, A::Error> {
        let mut row = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        let mut invalid = false;

        while let Some(number) = seq.next_element::<Number>()? {
            match number.0 {
                Some(value) => row.push(value),
                None => invalid = true,
            }
        }

        Ok(match invalid {
            true => Item::Invalid(MIXED_ARRAY),
            false => Item::Row(row),
        })
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Item, A::Error> {
        drain(map).map(|()| Item::Invalid(INVALID_ARRAY))
    }
}

/// A JSON number narrowed to `f32`, or `None` for anything that is not a
/// number. Values beyond `f32`'s range become infinity, matching the JS/Python
/// SDKs.
struct Number(Option<f32>);

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(NumberVisitor)
    }
}

struct NumberVisitor;

impl<'de> Visitor<'de> for NumberVisitor {
    type Value = Number;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a number")
    }

    fn visit_i64<E: DeError>(self, value: i64) -> Result<Number, E> {
        Ok(Number(Some(value as f32)))
    }

    fn visit_u64<E: DeError>(self, value: u64) -> Result<Number, E> {
        Ok(Number(Some(value as f32)))
    }

    fn visit_f64<E: DeError>(self, value: f64) -> Result<Number, E> {
        Ok(Number(Some(value as f32)))
    }

    fn visit_bool<E: DeError>(self, _: bool) -> Result<Number, E> {
        Ok(Number(None))
    }

    fn visit_unit<E: DeError>(self) -> Result<Number, E> {
        Ok(Number(None))
    }

    fn visit_none<E: DeError>(self) -> Result<Number, E> {
        Ok(Number(None))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Number, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_str<E: DeError>(self, _: &str) -> Result<Number, E> {
        Ok(Number(None))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Number, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Number(None))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Number, A::Error> {
        drain(map).map(|()| Number(None))
    }
}

/// An object key, classified as a sparse-vector index or a struct field name
/// while it is read — indices never reach the heap.
enum Key {
    Index(u32),
    Name(String),
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(KeyVisitor)
    }
}

struct KeyVisitor;

impl Visitor<'_> for KeyVisitor {
    type Value = Key;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object key")
    }

    fn visit_str<E: DeError>(self, value: &str) -> Result<Key, E> {
        Ok(match value.parse::<u32>() {
            Ok(index) => Key::Index(index),
            Err(_) => Key::Name(value.to_string()),
        })
    }

    fn visit_string<E: DeError>(self, value: String) -> Result<Key, E> {
        Ok(match value.parse::<u32>() {
            Ok(index) => Key::Index(index),
            Err(_) => Key::Name(value),
        })
    }
}

// Serialization

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ValueRef(&self.0).serialize(serializer)
    }
}

/// Serializes a borrowed [`TopkValue`] as JSON, without cloning it into an
/// intermediate JSON document.
pub struct ValueRef<'a>(pub &'a TopkValue);

impl Serialize for ValueRef<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match &self.0.value {
            None | Some(value::Value::Null(_)) => s.serialize_unit(),
            Some(value::Value::Bool(v)) => s.serialize_bool(*v),
            Some(value::Value::String(v)) => s.serialize_str(v),
            Some(value::Value::U32(v)) => s.serialize_u32(*v),
            Some(value::Value::U64(v)) => s.serialize_u64(*v),
            Some(value::Value::I32(v)) => s.serialize_i32(*v),
            Some(value::Value::I64(v)) => s.serialize_i64(*v),
            Some(value::Value::F32(v)) => s.serialize_f32(finite(*v)?),
            Some(value::Value::F64(v)) => s.serialize_f64(finite(*v)?),
            Some(value::Value::Binary(v)) => s.collect_seq(v.iter()),
            #[allow(deprecated)]
            Some(value::Value::Vector(v)) => match &v.vector {
                Some(vector::Vector::Float(v)) => Floats(&v.values).serialize(s),
                Some(vector::Vector::Byte(v)) => s.collect_seq(&v.values),
                None => s.serialize_unit(),
            },
            Some(value::Value::Struct(v)) => {
                s.collect_map(v.fields.iter().map(|(key, value)| (key, ValueRef(value))))
            }
            Some(value::Value::List(v)) => match &v.values {
                Some(list::Values::U8(v)) => s.collect_seq(&v.values),
                Some(list::Values::I8(v)) => s.collect_seq(v.as_ref()),
                Some(list::Values::U32(v)) => s.collect_seq(&v.values),
                Some(list::Values::U64(v)) => s.collect_seq(&v.values),
                Some(list::Values::I32(v)) => s.collect_seq(&v.values),
                Some(list::Values::I64(v)) => s.collect_seq(&v.values),
                Some(list::Values::F8(v)) => Floats(v.as_ref()).serialize(s),
                Some(list::Values::F16(v)) => Floats(v.as_ref()).serialize(s),
                Some(list::Values::F32(v)) => Floats(&v.values).serialize(s),
                Some(list::Values::F64(v)) => Doubles(&v.values).serialize(s),
                Some(list::Values::String(v)) => s.collect_seq(&v.values),
                None => s.collect_seq(std::iter::empty::<f32>()),
            },
            Some(value::Value::SparseVector(v)) => {
                let mut map = s.serialize_map(Some(v.indices.len()))?;
                match &v.values {
                    Some(sparse_vector::Values::F32(values)) => {
                        for (index, value) in v.indices.iter().zip(&values.values) {
                            map.serialize_entry(&Index(*index), &finite(*value)?)?;
                        }
                    }
                    Some(sparse_vector::Values::F16(values)) => {
                        for (index, value) in v.indices.iter().zip(values.as_ref()) {
                            map.serialize_entry(&Index(*index), &finite(f32::from(*value))?)?;
                        }
                    }
                    Some(sparse_vector::Values::F8(values)) => {
                        for (index, value) in v.indices.iter().zip(values.as_ref()) {
                            map.serialize_entry(&Index(*index), &finite(f32::from(*value))?)?;
                        }
                    }
                    Some(sparse_vector::Values::U8(values)) => {
                        for (index, value) in v.indices.iter().zip(&values.values) {
                            map.serialize_entry(&Index(*index), value)?;
                        }
                    }
                    Some(sparse_vector::Values::I8(values)) => {
                        for (index, value) in v.indices.iter().zip(values.as_ref()) {
                            map.serialize_entry(&Index(*index), value)?;
                        }
                    }
                    None => {}
                }
                map.end()
            }
            Some(value::Value::Matrix(v)) => {
                let num_cols = v.num_cols as usize;
                match &v.values {
                    None => s.collect_seq(std::iter::empty::<f32>()),
                    Some(_) if num_cols == 0 => s.collect_seq(std::iter::empty::<f32>()),
                    Some(matrix::Values::F32(m)) => {
                        s.collect_seq(m.values.chunks(num_cols).map(Floats))
                    }
                    Some(matrix::Values::F16(m)) => {
                        s.collect_seq(m.as_ref().chunks(num_cols).map(Floats))
                    }
                    Some(matrix::Values::F8(m)) => {
                        s.collect_seq(m.as_ref().chunks(num_cols).map(Floats))
                    }
                    Some(matrix::Values::U8(m)) => s.collect_seq(m.values.chunks(num_cols)),
                    Some(matrix::Values::I8(m)) => s.collect_seq(m.as_ref().chunks(num_cols)),
                }
            }
        }
    }
}

// JSON has no way to spell NaN or an infinity, and silently writing `null`
// would hide the value.
fn finite<T: Into<f64> + Copy, E: SerError>(value: T) -> Result<T, E> {
    match value.into().is_finite() {
        true => Ok(value),
        false => Err(SerError::custom("non-finite floating-point value")),
    }
}

/// A slice of anything that widens to `f32`, serialized as a JSON array.
struct Floats<'a, T>(&'a [T]);

impl<T: Copy + Into<f32>> Serialize for Floats<'_, T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            seq.serialize_element(&finite::<f32, S::Error>((*value).into())?)?;
        }
        seq.end()
    }
}

struct Doubles<'a>(&'a [f64]);

impl Serialize for Doubles<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            seq.serialize_element(&finite::<f64, S::Error>(*value)?)?;
        }
        seq.end()
    }
}

/// A sparse-vector index as an object key, formatted in place rather than
/// through a `String`.
struct Index(u32);

impl Serialize for Index {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut buf = [0u8; 10];
        let mut end = buf.len();
        let mut rest = self.0;
        loop {
            end -= 1;
            buf[end] = b'0' + (rest % 10) as u8;
            rest /= 10;
            if rest == 0 {
                break;
            }
        }

        s.serialize_str(std::str::from_utf8(&buf[end..]).expect("decimal digits are ASCII"))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    fn f16s(values: &[f32]) -> Vec<half::f16> {
        values.iter().copied().map(half::f16::from_f32).collect()
    }

    fn f8s(values: &[f32]) -> Vec<float8::F8E4M3> {
        values
            .iter()
            .copied()
            .map(float8::F8E4M3::from_f32)
            .collect()
    }

    fn from_json_value(input: serde_json::Value) -> Result<TopkValue, serde_json::Error> {
        serde_json::from_value::<Value>(input).map(Value::into_inner)
    }

    #[rstest]
    // scalars
    #[case::null(json!(null), TopkValue::null())]
    #[case::bool(json!(true), TopkValue::bool(true))]
    #[case::string(json!("a"), TopkValue::string("a"))]
    #[case::i64(json!(42), TopkValue::i64(42))]
    #[case::i64_negative(json!(-5), TopkValue::i64(-5))]
    #[case::u64(
        serde_json::Value::Number(serde_json::Number::from(i64::MAX as u64 + 1)),
        TopkValue::u64(i64::MAX as u64 + 1)
    )]
    #[case::f64(json!(0.1), TopkValue::f64(0.1))]
    // arrays
    #[case::int_list(json!([1, 2]), TopkValue::list(vec![1_i64, 2]))]
    #[case::negative_int_list(json!([-1, 2]), TopkValue::list(vec![-1_i64, 2]))]
    #[case::byte_value_list(json!([255, 128, 1, 0]), TopkValue::list(vec![255_i64, 128, 1, 0]))]
    #[case::whole_float_list(json!([1.0, 2.0]), TopkValue::list(vec![1.0_f32, 2.0]))]
    #[case::mixed_list(json!([1, 2.5]), TopkValue::list(vec![1.0_f32, 2.5]))]
    #[case::float_then_int_list(json!([2.5, 1]), TopkValue::list(vec![2.5_f32, 1.0]))]
    #[case::u64_overflow_list(
        json!([1, i64::MAX as u64 + 1]),
        TopkValue::list(vec![1_u64, i64::MAX as u64 + 1])
    )]
    #[case::u64_overflow_then_int_list(
        json!([i64::MAX as u64 + 1, 1]),
        TopkValue::list(vec![i64::MAX as u64 + 1, 1_u64])
    )]
    // A negative and a value past `i64::MAX` share no integer width.
    #[case::u64_overflow_with_negative_list(
        json!([-1, i64::MAX as u64 + 1]),
        TopkValue::list(vec![-1.0_f32, (i64::MAX as u64 + 1) as f32])
    )]
    #[case::u64_overflow_then_negative_list(
        json!([i64::MAX as u64 + 1, -1]),
        TopkValue::list(vec![(i64::MAX as u64 + 1) as f32, -1.0])
    )]
    #[case::empty_list(json!([]), TopkValue::list(Vec::<f32>::new()))]
    #[case::string_list(json!(["a", "b"]), TopkValue::list(vec!["a", "b"]))]
    // matrices
    #[case::matrix(
        json!([[1, 2], [3.5, 4]]),
        TopkValue::matrix(2, vec![1.0_f32, 2.0, 3.5, 4.0])
    )]
    // objects
    #[case::sparse_vector(
        json!({"0": 1.5, "2": 3.0}),
        TopkValue::f32_sparse_vector(vec![0, 2], vec![1.5, 3.0])
    )]
    #[case::struct_value(
        json!({"name": "a", "count": 2}),
        TopkValue::r#struct([("name", TopkValue::string("a")), ("count", TopkValue::i64(2))])
    )]
    #[case::empty_object(
        json!({}),
        TopkValue::r#struct(Vec::<(String, TopkValue)>::new())
    )]
    #[case::non_u32_index_object(
        json!({"4294967296": 1}),
        TopkValue::r#struct([("4294967296", TopkValue::i64(1))])
    )]
    #[case::nested_struct(
        json!({"a": {"b": [1, 2]}}),
        TopkValue::r#struct([(
            "a",
            TopkValue::r#struct([("b", TopkValue::list(vec![1_i64, 2]))])
        )])
    )]
    fn from_json(#[case] input: serde_json::Value, #[case] expected: TopkValue) {
        assert_eq!(from_json_value(input.clone()).unwrap(), expected);

        // The same shapes off the wire, through a streaming parser.
        assert_eq!(
            serde_json::from_str::<Value>(&input.to_string())
                .unwrap()
                .into_inner(),
            expected
        );
    }

    #[rstest]
    #[case::mixed_number_string(json!([1, "a"]))]
    #[case::mixed_string_number(json!(["a", 1]))]
    #[case::bool_array(json!([true, false]))]
    #[case::null_array(json!([null]))]
    #[case::object_array(json!([{}]))]
    #[case::nested_object_array(json!([[{}]]))]
    #[case::ragged_matrix(json!([[1], [2, 3]]))]
    #[case::zero_column_matrix(json!([[]]))]
    #[case::non_numeric_matrix(json!([[1], ["a"]]))]
    #[case::mixed_matrix_scalar(json!([[1], 2]))]
    #[case::mixed_scalar_matrix(json!([1, [2]]))]
    #[case::mixed_keys_object(json!({"0": 1.5, "name": 2}))]
    #[case::mixed_keys_object_reversed(json!({"name": 2, "0": 1.5}))]
    #[case::numeric_key_non_number_value(json!({"0": "a"}))]
    #[case::nested_invalid_value(json!({"a": {"b": [1, "c"]}}))]
    fn from_json_invalid(#[case] input: serde_json::Value) {
        assert!(from_json_value(input.clone()).is_err());
        assert!(serde_json::from_str::<Value>(&input.to_string()).is_err());
    }

    // Shape errors are reported to the caller, not to the deserializer, so the
    // rest of the document still parses.
    #[rstest]
    #[case::mixed_array(json!({"ok": 1, "bad": [1, "a"], "also_ok": 2}))]
    #[case::numeric_field_name(json!({"ok": 1, "bad": {"0": "a"}}))]
    fn lenient_captures_shape_errors(#[case] input: serde_json::Value) {
        let value: HashMap<String, LenientValue> = serde_json::from_value(input).unwrap();

        assert!(value["ok"].0.is_ok());
        assert!(value["bad"].0.is_err());
    }

    #[test]
    fn lenient_still_fails_on_malformed_json() {
        assert!(serde_json::from_str::<LenientValue>("{\"a\":").is_err());
    }

    #[rstest]
    // scalars
    #[case::null(TopkValue::null(), json!(null))]
    #[case::bool(TopkValue::bool(true), json!(true))]
    #[case::string(TopkValue::string("a"), json!("a"))]
    #[case::u32(TopkValue::u32(7), json!(7))]
    #[case::u64(TopkValue::u64(i64::MAX as u64 + 1), json!(i64::MAX as u64 + 1))]
    #[case::i32(TopkValue::i32(-7), json!(-7))]
    #[case::i64(TopkValue::i64(-8), json!(-8))]
    #[case::f64(TopkValue::f64(0.5), json!(0.5))]
    // binary
    #[case::binary(TopkValue::binary(vec![1_u8, 2, 3]), json!([1, 2, 3]))]
    // lists
    #[case::u8_list(TopkValue::list(vec![1_u8, 2]), json!([1, 2]))]
    #[case::i8_list(TopkValue::list(vec![-1_i8, 2]), json!([-1, 2]))]
    #[case::u32_list(TopkValue::list(vec![1_u32, 2]), json!([1, 2]))]
    #[case::i32_list(TopkValue::list(vec![-1_i32, 2]), json!([-1, 2]))]
    #[case::u64_list(TopkValue::list(vec![1_u64, 2]), json!([1, 2]))]
    #[case::i64_list(TopkValue::list(vec![1_i64, 2]), json!([1, 2]))]
    #[case::f8_list(TopkValue::list(f8s(&[1.5, 2.5])), json!([1.5, 2.5]))]
    #[case::f16_list(TopkValue::list(f16s(&[1.5, 2.5])), json!([1.5, 2.5]))]
    #[case::f32_list(TopkValue::list(vec![1.5_f32, 2.5]), json!([1.5, 2.5]))]
    #[case::f64_list(TopkValue::list(vec![0.5_f64, 1.25]), json!([0.5, 1.25]))]
    #[case::string_list(TopkValue::list(vec!["a", "b"]), json!(["a", "b"]))]
    // sparse vectors
    #[case::f32_sparse_vector(
        TopkValue::f32_sparse_vector(vec![0, 2], vec![1.5, 3.0]),
        json!({"0": 1.5, "2": 3.0})
    )]
    #[case::f16_sparse_vector(
        TopkValue::f16_sparse_vector(vec![0, 2], f16s(&[1.5, 3.0])),
        json!({"0": 1.5, "2": 3.0})
    )]
    #[case::f8_sparse_vector(
        TopkValue::f8_sparse_vector(vec![0, 2], f8s(&[1.5, 3.0])),
        json!({"0": 1.5, "2": 3.0})
    )]
    #[case::u8_sparse_vector(
        TopkValue::u8_sparse_vector(vec![0, 2], vec![1, 3]),
        json!({"0": 1, "2": 3})
    )]
    #[case::i8_sparse_vector(
        TopkValue::i8_sparse_vector(vec![0, 2], vec![-1, 3]),
        json!({"0": -1, "2": 3})
    )]
    #[case::wide_sparse_index(
        TopkValue::f32_sparse_vector(vec![4294967295], vec![1.5]),
        json!({"4294967295": 1.5})
    )]
    // matrices
    #[case::f32_matrix(
        TopkValue::matrix(2, vec![1.5_f32, 2.5, 3.5, 4.5]),
        json!([[1.5, 2.5], [3.5, 4.5]])
    )]
    #[case::f16_matrix(
        TopkValue::matrix(2, f16s(&[1.5, 2.5, 3.5, 4.5])),
        json!([[1.5, 2.5], [3.5, 4.5]])
    )]
    #[case::f8_matrix(
        TopkValue::matrix(2, f8s(&[1.5, 2.5, 3.5, 4.5])),
        json!([[1.5, 2.5], [3.5, 4.5]])
    )]
    #[case::u8_matrix(
        TopkValue::matrix(2, vec![1_u8, 2, 3, 4]),
        json!([[1, 2], [3, 4]])
    )]
    #[case::i8_matrix(
        TopkValue::matrix(2, vec![-1_i8, 2, -3, 4]),
        json!([[-1, 2], [-3, 4]])
    )]
    // structs
    #[case::struct_value(
        TopkValue::r#struct([("name", TopkValue::string("a")), ("count", TopkValue::i64(2))]),
        json!({"name": "a", "count": 2})
    )]
    #[case::empty_struct(
        TopkValue::r#struct(Vec::<(String, TopkValue)>::new()),
        json!({})
    )]
    fn to_json(#[case] input: TopkValue, #[case] expected: serde_json::Value) {
        assert_eq!(serde_json::to_value(Value(input)).unwrap(), expected);
    }

    #[rstest]
    #[case::nan_f32(TopkValue::f32(f32::NAN))]
    #[case::inf_f64(TopkValue::f64(f64::INFINITY))]
    #[case::nan_in_list(TopkValue::list(vec![1.0_f32, f32::NAN]))]
    #[case::inf_in_matrix(TopkValue::matrix(1, vec![f32::INFINITY]))]
    #[case::nan_in_sparse_vector(TopkValue::f32_sparse_vector(vec![0], vec![f32::NAN]))]
    fn non_finite(#[case] input: TopkValue) {
        assert!(serde_json::to_value(Value(input)).is_err());
    }

    #[rstest]
    #[case::null(TopkValue::null(), json!(null))]
    #[case::f32_list(TopkValue::list(vec![1.5_f32, 2.5]), json!([1.5, 2.5]))]
    #[case::struct_value(
        TopkValue::r#struct([("name", TopkValue::string("a")), ("count", TopkValue::i64(2))]),
        json!({"name": "a", "count": 2})
    )]
    fn serde_roundtrip(#[case] input: TopkValue, #[case] expected: serde_json::Value) {
        let serialized = serde_json::to_value(Value(input.clone())).unwrap();
        assert_eq!(serialized, expected);
        assert_eq!(from_json_value(serialized).unwrap(), input);
    }
}
