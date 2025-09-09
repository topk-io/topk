// Automatically generated rust module for 'value.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![allow(deprecated)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use std::borrow::Cow;
use std::collections::HashMap;
type KVMap<K, V> = HashMap<K, V>;
use quick_protobuf::{MessageInfo, MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Value<'a> {
    pub value: data::v1::mod_Value::OneOfvalue<'a>,
}

impl<'a> MessageRead<'a> for Value<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.value = data::v1::mod_Value::OneOfvalue::bool_pb(r.read_bool(bytes)?),
                Ok(32) => msg.value = data::v1::mod_Value::OneOfvalue::u32_pb(r.read_uint32(bytes)?),
                Ok(40) => msg.value = data::v1::mod_Value::OneOfvalue::u64_pb(r.read_uint64(bytes)?),
                Ok(64) => msg.value = data::v1::mod_Value::OneOfvalue::i32_pb(r.read_int32(bytes)?),
                Ok(72) => msg.value = data::v1::mod_Value::OneOfvalue::i64_pb(r.read_int64(bytes)?),
                Ok(85) => msg.value = data::v1::mod_Value::OneOfvalue::f32_pb(r.read_float(bytes)?),
                Ok(89) => msg.value = data::v1::mod_Value::OneOfvalue::f64_pb(r.read_double(bytes)?),
                Ok(98) => msg.value = data::v1::mod_Value::OneOfvalue::string(r.read_string(bytes).map(Cow::Borrowed)?),
                Ok(106) => msg.value = data::v1::mod_Value::OneOfvalue::binary(r.read_bytes(bytes).map(Cow::Borrowed)?),
                Ok(114) => msg.value = data::v1::mod_Value::OneOfvalue::vector(r.read_message::<data::v1::Vector>(bytes)?),
                Ok(122) => msg.value = data::v1::mod_Value::OneOfvalue::null(r.read_message::<data::v1::Null>(bytes)?),
                Ok(130) => msg.value = data::v1::mod_Value::OneOfvalue::sparse_vector(r.read_message::<data::v1::SparseVector>(bytes)?),
                Ok(138) => msg.value = data::v1::mod_Value::OneOfvalue::list(r.read_message::<data::v1::List>(bytes)?),
                Ok(146) => msg.value = data::v1::mod_Value::OneOfvalue::struct_pb(r.read_message::<data::v1::Struct>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Value<'a> {
    fn get_size(&self) -> usize {
        0
        + match self.value {
            data::v1::mod_Value::OneOfvalue::bool_pb(ref m) => 1 + sizeof_varint(*(m) as u64),
            data::v1::mod_Value::OneOfvalue::u32_pb(ref m) => 1 + sizeof_varint(*(m) as u64),
            data::v1::mod_Value::OneOfvalue::u64_pb(ref m) => 1 + sizeof_varint(*(m) as u64),
            data::v1::mod_Value::OneOfvalue::i32_pb(ref m) => 1 + sizeof_varint(*(m) as u64),
            data::v1::mod_Value::OneOfvalue::i64_pb(ref m) => 1 + sizeof_varint(*(m) as u64),
            data::v1::mod_Value::OneOfvalue::f32_pb(_) => 1 + 4,
            data::v1::mod_Value::OneOfvalue::f64_pb(_) => 1 + 8,
            data::v1::mod_Value::OneOfvalue::string(ref m) => 1 + sizeof_len((m).len()),
            data::v1::mod_Value::OneOfvalue::binary(ref m) => 1 + sizeof_len((m).len()),
            data::v1::mod_Value::OneOfvalue::vector(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_Value::OneOfvalue::null(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_Value::OneOfvalue::sparse_vector(ref m) => 2 + sizeof_len((m).get_size()),
            data::v1::mod_Value::OneOfvalue::list(ref m) => 2 + sizeof_len((m).get_size()),
            data::v1::mod_Value::OneOfvalue::struct_pb(ref m) => 2 + sizeof_len((m).get_size()),
            data::v1::mod_Value::OneOfvalue::None => 0,
    }    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        match self.value {            data::v1::mod_Value::OneOfvalue::bool_pb(ref m) => { w.write_with_tag(8, |w| w.write_bool(*m))? },
            data::v1::mod_Value::OneOfvalue::u32_pb(ref m) => { w.write_with_tag(32, |w| w.write_uint32(*m))? },
            data::v1::mod_Value::OneOfvalue::u64_pb(ref m) => { w.write_with_tag(40, |w| w.write_uint64(*m))? },
            data::v1::mod_Value::OneOfvalue::i32_pb(ref m) => { w.write_with_tag(64, |w| w.write_int32(*m))? },
            data::v1::mod_Value::OneOfvalue::i64_pb(ref m) => { w.write_with_tag(72, |w| w.write_int64(*m))? },
            data::v1::mod_Value::OneOfvalue::f32_pb(ref m) => { w.write_with_tag(85, |w| w.write_float(*m))? },
            data::v1::mod_Value::OneOfvalue::f64_pb(ref m) => { w.write_with_tag(89, |w| w.write_double(*m))? },
            data::v1::mod_Value::OneOfvalue::string(ref m) => { w.write_with_tag(98, |w| w.write_string(&**m))? },
            data::v1::mod_Value::OneOfvalue::binary(ref m) => { w.write_with_tag(106, |w| w.write_bytes(&**m))? },
            data::v1::mod_Value::OneOfvalue::vector(ref m) => { w.write_with_tag(114, |w| w.write_message(m))? },
            data::v1::mod_Value::OneOfvalue::null(ref m) => { w.write_with_tag(122, |w| w.write_message(m))? },
            data::v1::mod_Value::OneOfvalue::sparse_vector(ref m) => { w.write_with_tag(130, |w| w.write_message(m))? },
            data::v1::mod_Value::OneOfvalue::list(ref m) => { w.write_with_tag(138, |w| w.write_message(m))? },
            data::v1::mod_Value::OneOfvalue::struct_pb(ref m) => { w.write_with_tag(146, |w| w.write_message(m))? },
            data::v1::mod_Value::OneOfvalue::None => {},
    }        Ok(())
    }
}
impl<'a> MessageInfo for Value<'a> {
    const PATH : &'static str = "topk.data.v1.Value";
}


pub mod mod_Value {

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum OneOfvalue<'a> {
    bool_pb(bool),
    u32_pb(u32),
    u64_pb(u64),
    i32_pb(i32),
    i64_pb(i64),
    f32_pb(f32),
    f64_pb(f64),
    string(Cow<'a, str>),
    binary(Cow<'a, [u8]>),
    #[deprecated]
    vector(data::v1::Vector<'a>),
    null(data::v1::Null),
    sparse_vector(data::v1::SparseVector<'a>),
    list(data::v1::List<'a>),
    struct_pb(data::v1::Struct<'a>),
    None,
}

impl<'a> Default for OneOfvalue<'a> {
    fn default() -> Self {
        OneOfvalue::None
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct List<'a> {
    pub values: data::v1::mod_List::OneOfvalues<'a>,
}

impl<'a> MessageRead<'a> for List<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.values = data::v1::mod_List::OneOfvalues::u32_pb(r.read_message::<data::v1::mod_List::U32>(bytes)?),
                Ok(18) => msg.values = data::v1::mod_List::OneOfvalues::u64_pb(r.read_message::<data::v1::mod_List::U64>(bytes)?),
                Ok(26) => msg.values = data::v1::mod_List::OneOfvalues::i32_pb(r.read_message::<data::v1::mod_List::I32>(bytes)?),
                Ok(34) => msg.values = data::v1::mod_List::OneOfvalues::i64_pb(r.read_message::<data::v1::mod_List::I64>(bytes)?),
                Ok(42) => msg.values = data::v1::mod_List::OneOfvalues::f32_pb(r.read_message::<data::v1::mod_List::F32>(bytes)?),
                Ok(50) => msg.values = data::v1::mod_List::OneOfvalues::f64_pb(r.read_message::<data::v1::mod_List::F64>(bytes)?),
                Ok(58) => msg.values = data::v1::mod_List::OneOfvalues::string(r.read_message::<data::v1::mod_List::String_pb>(bytes)?),
                Ok(66) => msg.values = data::v1::mod_List::OneOfvalues::u8_pb(r.read_message::<data::v1::mod_List::U8>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for List<'a> {
    fn get_size(&self) -> usize {
        0
        + match self.values {
            data::v1::mod_List::OneOfvalues::u32_pb(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_List::OneOfvalues::u64_pb(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_List::OneOfvalues::i32_pb(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_List::OneOfvalues::i64_pb(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_List::OneOfvalues::f32_pb(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_List::OneOfvalues::f64_pb(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_List::OneOfvalues::string(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_List::OneOfvalues::u8_pb(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_List::OneOfvalues::None => 0,
    }    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        match self.values {            data::v1::mod_List::OneOfvalues::u32_pb(ref m) => { w.write_with_tag(10, |w| w.write_message(m))? },
            data::v1::mod_List::OneOfvalues::u64_pb(ref m) => { w.write_with_tag(18, |w| w.write_message(m))? },
            data::v1::mod_List::OneOfvalues::i32_pb(ref m) => { w.write_with_tag(26, |w| w.write_message(m))? },
            data::v1::mod_List::OneOfvalues::i64_pb(ref m) => { w.write_with_tag(34, |w| w.write_message(m))? },
            data::v1::mod_List::OneOfvalues::f32_pb(ref m) => { w.write_with_tag(42, |w| w.write_message(m))? },
            data::v1::mod_List::OneOfvalues::f64_pb(ref m) => { w.write_with_tag(50, |w| w.write_message(m))? },
            data::v1::mod_List::OneOfvalues::string(ref m) => { w.write_with_tag(58, |w| w.write_message(m))? },
            data::v1::mod_List::OneOfvalues::u8_pb(ref m) => { w.write_with_tag(66, |w| w.write_message(m))? },
            data::v1::mod_List::OneOfvalues::None => {},
    }        Ok(())
    }
}
impl<'a> MessageInfo for List<'a> {
    const PATH : &'static str = "topk.data.v1.List";
}


pub mod mod_List {

use std::borrow::Cow;
use super::*;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct U8<'a> {
    pub values: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for U8<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.values = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for U8<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.values == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.values).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.values != Cow::Borrowed(b"") { w.write_with_tag(10, |w| w.write_bytes(&**&self.values))?; }
        Ok(())
    }
}
impl<'a> MessageInfo for U8<'a> {
    const PATH : &'static str = "topk.data.v1.mod_List.U8";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct U32 {
    pub values: Vec<u32>,
}

impl<'a> MessageRead<'a> for U32 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.values.push(r.read_uint32(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for U32 {
    fn get_size(&self) -> usize {
        0
        + self.values.iter().map(|s| 1 + sizeof_varint(*(s) as u64)).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.values { w.write_with_tag(8, |w| w.write_uint32(*s))?; }
        Ok(())
    }
}
impl MessageInfo for U32 {
    const PATH : &'static str = "topk.data.v1.mod_List.U32";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct U64 {
    pub values: Vec<u64>,
}

impl<'a> MessageRead<'a> for U64 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.values.push(r.read_uint64(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for U64 {
    fn get_size(&self) -> usize {
        0
        + self.values.iter().map(|s| 1 + sizeof_varint(*(s) as u64)).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.values { w.write_with_tag(8, |w| w.write_uint64(*s))?; }
        Ok(())
    }
}
impl MessageInfo for U64 {
    const PATH : &'static str = "topk.data.v1.mod_List.U64";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct I32 {
    pub values: Vec<i32>,
}

impl<'a> MessageRead<'a> for I32 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.values.push(r.read_int32(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for I32 {
    fn get_size(&self) -> usize {
        0
        + self.values.iter().map(|s| 1 + sizeof_varint(*(s) as u64)).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.values { w.write_with_tag(8, |w| w.write_int32(*s))?; }
        Ok(())
    }
}
impl MessageInfo for I32 {
    const PATH : &'static str = "topk.data.v1.mod_List.I32";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct I64 {
    pub values: Vec<i64>,
}

impl<'a> MessageRead<'a> for I64 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.values.push(r.read_int64(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for I64 {
    fn get_size(&self) -> usize {
        0
        + self.values.iter().map(|s| 1 + sizeof_varint(*(s) as u64)).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.values { w.write_with_tag(8, |w| w.write_int64(*s))?; }
        Ok(())
    }
}
impl MessageInfo for I64 {
    const PATH : &'static str = "topk.data.v1.mod_List.I64";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct F32 {
    pub values: Vec<f32>,
}

impl<'a> MessageRead<'a> for F32 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(13) => msg.values.push(r.read_float(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for F32 {
    fn get_size(&self) -> usize {
        0
        + (1 + 4) * self.values.len()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.values { w.write_with_tag(13, |w| w.write_float(*s))?; }
        Ok(())
    }
}
impl MessageInfo for F32 {
    const PATH : &'static str = "topk.data.v1.mod_List.F32";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct F64 {
    pub values: Vec<f64>,
}

impl<'a> MessageRead<'a> for F64 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(9) => msg.values.push(r.read_double(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for F64 {
    fn get_size(&self) -> usize {
        0
        + (1 + 8) * self.values.len()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.values { w.write_with_tag(9, |w| w.write_double(*s))?; }
        Ok(())
    }
}
impl MessageInfo for F64 {
    const PATH : &'static str = "topk.data.v1.mod_List.F64";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct String_pb<'a> {
    pub values: Vec<Cow<'a, str>>,
}

impl<'a> MessageRead<'a> for String_pb<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.values.push(r.read_string(bytes).map(Cow::Borrowed)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for String_pb<'a> {
    fn get_size(&self) -> usize {
        0
        + self.values.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.values { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        Ok(())
    }
}
impl<'a> MessageInfo for String_pb<'a> {
    const PATH : &'static str = "topk.data.v1.mod_List.String_pb";
}


#[derive(Debug, PartialEq, Clone)]
pub enum OneOfvalues<'a> {
    u32_pb(data::v1::mod_List::U32),
    u64_pb(data::v1::mod_List::U64),
    i32_pb(data::v1::mod_List::I32),
    i64_pb(data::v1::mod_List::I64),
    f32_pb(data::v1::mod_List::F32),
    f64_pb(data::v1::mod_List::F64),
    string(data::v1::mod_List::String_pb<'a>),
    u8_pb(data::v1::mod_List::U8<'a>),
    None,
}

impl<'a> Default for OneOfvalues<'a> {
    fn default() -> Self {
        OneOfvalues::None
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Struct<'a> {
    pub fields: KVMap<Cow<'a, str>, data::v1::Value<'a>>,
}

impl<'a> MessageRead<'a> for Struct<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => {
                    let (key, value) = r.read_map(bytes, |r, bytes| Ok(r.read_string(bytes).map(Cow::Borrowed)?), |r, bytes| Ok(r.read_message::<data::v1::Value>(bytes)?))?;
                    msg.fields.insert(key, value);
                }
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Struct<'a> {
    fn get_size(&self) -> usize {
        0
        + self.fields.iter().map(|(k, v)| 1 + sizeof_len(2 + sizeof_len((k).len()) + sizeof_len((v).get_size()))).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for (k, v) in self.fields.iter() { w.write_with_tag(10, |w| w.write_map(2 + sizeof_len((k).len()) + sizeof_len((v).get_size()), 10, |w| w.write_string(&**k), 18, |w| w.write_message(v)))?; }
        Ok(())
    }
}
impl<'a> MessageInfo for Struct<'a> {
    const PATH : &'static str = "topk.data.v1.Struct";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Vector<'a> {
    pub vector: data::v1::mod_Vector::OneOfvector<'a>,
}

impl<'a> MessageRead<'a> for Vector<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.vector = data::v1::mod_Vector::OneOfvector::float(r.read_message::<data::v1::mod_Vector::Float>(bytes)?),
                Ok(18) => msg.vector = data::v1::mod_Vector::OneOfvector::byte(r.read_message::<data::v1::mod_Vector::Byte>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Vector<'a> {
    fn get_size(&self) -> usize {
        0
        + match self.vector {
            data::v1::mod_Vector::OneOfvector::float(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_Vector::OneOfvector::byte(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_Vector::OneOfvector::None => 0,
    }    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        match self.vector {            data::v1::mod_Vector::OneOfvector::float(ref m) => { w.write_with_tag(10, |w| w.write_message(m))? },
            data::v1::mod_Vector::OneOfvector::byte(ref m) => { w.write_with_tag(18, |w| w.write_message(m))? },
            data::v1::mod_Vector::OneOfvector::None => {},
    }        Ok(())
    }
}
impl<'a> MessageInfo for Vector<'a> {
    const PATH : &'static str = "topk.data.v1.Vector";
}


pub mod mod_Vector {

use std::borrow::Cow;
use super::*;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Float {
    #[deprecated]
    pub values: Vec<f32>,
}

impl<'a> MessageRead<'a> for Float {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(13) => msg.values.push(r.read_float(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Float {
    fn get_size(&self) -> usize {
        0
        + (1 + 4) * self.values.len()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.values { w.write_with_tag(13, |w| w.write_float(*s))?; }
        Ok(())
    }
}
impl MessageInfo for Float {
    const PATH : &'static str = "topk.data.v1.mod_Vector.Float";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Byte<'a> {
    #[deprecated]
    pub values: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for Byte<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.values = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Byte<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.values == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.values).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.values != Cow::Borrowed(b"") { w.write_with_tag(10, |w| w.write_bytes(&**&self.values))?; }
        Ok(())
    }
}
impl<'a> MessageInfo for Byte<'a> {
    const PATH : &'static str = "topk.data.v1.mod_Vector.Byte";
}


#[derive(Debug, PartialEq, Clone)]
pub enum OneOfvector<'a> {
    #[deprecated]
    float(data::v1::mod_Vector::Float),
    #[deprecated]
    byte(data::v1::mod_Vector::Byte<'a>),
    None,
}

impl<'a> Default for OneOfvector<'a> {
    fn default() -> Self {
        OneOfvector::None
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct SparseVector<'a> {
    pub indices: Vec<u32>,
    pub values: data::v1::mod_SparseVector::OneOfvalues<'a>,
}

impl<'a> MessageRead<'a> for SparseVector<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.indices = r.read_packed(bytes, |r, bytes| Ok(r.read_uint32(bytes)?))?,
                Ok(18) => msg.values = data::v1::mod_SparseVector::OneOfvalues::f32_pb(r.read_message::<data::v1::mod_SparseVector::F32Values>(bytes)?),
                Ok(26) => msg.values = data::v1::mod_SparseVector::OneOfvalues::u8_pb(r.read_message::<data::v1::mod_SparseVector::U8Values>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for SparseVector<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.indices.is_empty() { 0 } else { 1 + sizeof_len(self.indices.iter().map(|s| sizeof_varint(*(s) as u64)).sum::<usize>()) }
        + match self.values {
            data::v1::mod_SparseVector::OneOfvalues::f32_pb(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_SparseVector::OneOfvalues::u8_pb(ref m) => 1 + sizeof_len((m).get_size()),
            data::v1::mod_SparseVector::OneOfvalues::None => 0,
    }    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_packed_with_tag(10, &self.indices, |w, m| w.write_uint32(*m), &|m| sizeof_varint(*(m) as u64))?;
        match self.values {            data::v1::mod_SparseVector::OneOfvalues::f32_pb(ref m) => { w.write_with_tag(18, |w| w.write_message(m))? },
            data::v1::mod_SparseVector::OneOfvalues::u8_pb(ref m) => { w.write_with_tag(26, |w| w.write_message(m))? },
            data::v1::mod_SparseVector::OneOfvalues::None => {},
    }        Ok(())
    }
}
impl<'a> MessageInfo for SparseVector<'a> {
    const PATH : &'static str = "topk.data.v1.SparseVector";
}


pub mod mod_SparseVector {

use std::borrow::Cow;
use super::*;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct F32Values {
    pub values: Vec<f32>,
}

impl<'a> MessageRead<'a> for F32Values {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(13) => msg.values.push(r.read_float(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for F32Values {
    fn get_size(&self) -> usize {
        0
        + (1 + 4) * self.values.len()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.values { w.write_with_tag(13, |w| w.write_float(*s))?; }
        Ok(())
    }
}
impl MessageInfo for F32Values {
    const PATH : &'static str = "topk.data.v1.mod_SparseVector.F32Values";
}


#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct U8Values<'a> {
    pub values: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for U8Values<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.values = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for U8Values<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.values == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.values).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.values != Cow::Borrowed(b"") { w.write_with_tag(10, |w| w.write_bytes(&**&self.values))?; }
        Ok(())
    }
}
impl<'a> MessageInfo for U8Values<'a> {
    const PATH : &'static str = "topk.data.v1.mod_SparseVector.U8Values";
}


#[derive(Debug, PartialEq, Clone)]
pub enum OneOfvalues<'a> {
    f32_pb(data::v1::mod_SparseVector::F32Values),
    u8_pb(data::v1::mod_SparseVector::U8Values<'a>),
    None,
}

impl<'a> Default for OneOfvalues<'a> {
    fn default() -> Self {
        OneOfvalues::None
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Null { }

impl<'a> MessageRead<'a> for Null {
    fn from_reader(r: &mut BytesReader, _: &[u8]) -> Result<Self> {
        r.read_to_end();
        Ok(Self::default())
    }
}

impl MessageWrite for Null { }
impl MessageInfo for Null {
    const PATH : &'static str = "topk.data.v1.Null";
}


