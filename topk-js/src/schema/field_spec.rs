use super::{data_type::DataType, field_index::FieldIndex};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(namespace = "schema")]
#[derive(Clone)]
pub struct FieldSpec {
    data_type: DataType,
    required: bool,
    index: Option<FieldIndex>,
}

impl FieldSpec {
    pub fn create(data_type: DataType) -> Self {
        Self {
            data_type,
            required: false,
            index: None,
        }
    }
}

#[napi(namespace = "schema")]
impl FieldSpec {
    #[napi]
    pub fn required(&self) -> Self {
        Self {
            required: true,
            ..self.clone()
        }
    }

    #[napi]
    pub fn index(&self, index: FieldIndex) -> Self {
        Self {
            index: Some(index),
            ..self.clone()
        }
    }
}

impl From<FieldSpec> for topk_rs::proto::v1::control::FieldSpec {
    fn from(field_spec: FieldSpec) -> Self {
        Self {
            data_type: Some(topk_rs::proto::v1::control::FieldType {
                data_type: Some(match field_spec.data_type {
                    DataType::Text => topk_rs::proto::v1::control::field_type::DataType::text(),
                    DataType::Integer => {
                        topk_rs::proto::v1::control::field_type::DataType::integer()
                    }
                    DataType::Float => topk_rs::proto::v1::control::field_type::DataType::float(),
                    DataType::Boolean => topk_rs::proto::v1::control::field_type::DataType::bool(),
                    DataType::F32Vector { dimension } => {
                        topk_rs::proto::v1::control::field_type::DataType::f32_vector(dimension)
                    }
                    DataType::U8Vector { dimension } => {
                        topk_rs::proto::v1::control::field_type::DataType::u8_vector(dimension)
                    }
                    DataType::BinaryVector { dimension } => {
                        topk_rs::proto::v1::control::field_type::DataType::binary_vector(dimension)
                    }
                    DataType::F32SparseVector => {
                        topk_rs::proto::v1::control::field_type::DataType::f32_sparse_vector()
                    }
                    DataType::U8SparseVector => {
                        topk_rs::proto::v1::control::field_type::DataType::u8_sparse_vector()
                    }
                    DataType::Bytes => topk_rs::proto::v1::control::field_type::DataType::bytes(),
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
        if let Ok(field_spec) = crate::try_cast_ref!(env, value, FieldSpec) {
            return Ok(field_spec.clone());
        }

        Err(napi::Error::from_reason("Value must be a FieldSpec"))
    }
}
