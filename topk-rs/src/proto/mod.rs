mod codec;
mod control;
mod ctx;
mod data;
mod macros;
pub mod v1 {
    pub use super::control::v1 as control;
    pub use super::ctx::v1 as ctx;
    pub use super::data::v1 as data;
}

#[cfg(test)]
mod tests {
    use prost::{
        encoding::{encode_key, encode_varint, WireType},
        Message,
    };

    use super::v1::control::{field_type_list::ListValueType, FieldType, FieldTypeList};
    use super::v1::data::Value;

    #[test]
    fn unknown_oneof_variant_decodes_as_none() {
        let mut buf = Vec::new();
        encode_key(9999, WireType::Varint, &mut buf);
        encode_varint(1, &mut buf);

        assert_eq!(Value::decode(&buf[..]).unwrap().value, None);
        assert_eq!(FieldType::decode(&buf[..]).unwrap().data_type, None);
    }

    #[test]
    fn unknown_enum_value_decodes_as_unspecified() {
        let mut buf = Vec::new();
        encode_key(1, WireType::Varint, &mut buf);
        encode_varint(9999, &mut buf);

        let list = FieldTypeList::decode(&buf[..]).unwrap();
        assert_eq!(list.value_type, 9999);
        assert_eq!(list.value_type(), ListValueType::Unspecified);
    }
}
