use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::{data_type::DataType, field_index::FieldIndex};

#[napi]
pub struct FieldSpec {
    data_type: DataType,
    required: bool,
    index: Option<FieldIndex>,
}

#[napi]
impl FieldSpec {
    pub fn new(data_type: DataType, required: bool, index: Option<FieldIndex>) -> Self {
        Self {
            data_type,
            required,
            index,
        }
    }
}

impl From<topk_protos::v1::control::FieldSpec> for FieldSpec {
    fn from(field_spec: topk_protos::v1::control::FieldSpec) -> Self {
        Self {
            data_type: DataType::from(field_spec.data_type.unwrap_or_default()),
            required: field_spec.required,
            index: field_spec.index.map(|idx| idx.into()),
        }
    }
}

impl From<FieldSpec> for topk_protos::v1::control::FieldSpec {
    fn from(field_spec: FieldSpec) -> Self {
        Self {
            data_type: Some(topk_protos::v1::control::FieldType {
                data_type: Some(match field_spec.data_type {
                    DataType::Text => {
                        topk_protos::v1::control::field_type::DataType::Text(Default::default())
                    }
                    DataType::Integer => {
                        topk_protos::v1::control::field_type::DataType::Integer(Default::default())
                    }
                    DataType::Float => {
                        topk_protos::v1::control::field_type::DataType::Float(Default::default())
                    }
                    DataType::Boolean => {
                        topk_protos::v1::control::field_type::DataType::Boolean(Default::default())
                    }
                    DataType::F32Vector => {
                        topk_protos::v1::control::field_type::DataType::F32Vector(Default::default())
                    }
                    DataType::U8Vector => {
                        topk_protos::v1::control::field_type::DataType::U8Vector(Default::default())
                    }
                    DataType::BinaryVector => {
                        topk_protos::v1::control::field_type::DataType::BinaryVector(
                            Default::default(),
                        )
                    }
                    DataType::Bytes => {
                        topk_protos::v1::control::field_type::DataType::Bytes(Default::default())
                    }
                }),
            }),
            required: field_spec.required,
            index: field_spec.index.map(|idx| idx.into()),
        }
    }
}

impl FromNapiValue for FieldSpec {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self> {
        todo!()

        // let data_type = DataType::from_napi_value(env, value)?;
        // let required = value.get_boolean()?;
        // let index = value.get_optional_field_index()?;

        // Ok(Self {
        //     data_type,
        //     required,
        //     index,
        // })
    }
}
