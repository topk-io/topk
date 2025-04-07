use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::{
    data_type::DataType,
    field_index::{FieldIndex, VectorDistanceMetric},
};

#[napi]
#[derive(Clone)]
pub struct FieldSpec {
    data_type: DataType,
    required: bool,
    index: Option<FieldIndex>,
}

#[napi]
impl FieldSpec {
    pub fn new(data_type: DataType) -> Self {
        Self {
            data_type,
            required: false,
            index: None,
        }
    }

    #[napi]
    pub fn required(&self) -> Self {
        Self {
            required: true,
            ..self.clone()
        }
    }

    #[napi]
    pub fn optional(&self) -> Self {
        Self {
            required: false,
            ..self.clone()
        }
    }

    #[napi]
    pub fn keyword_index(&self) -> Self {
        self.index(FieldIndex::KeywordIndex)
    }

    #[napi]
    pub fn vector_index(&self, metric: VectorDistanceMetric) -> Self {
        self.index(FieldIndex::VectorIndex { metric })
    }

    #[napi]
    pub fn semantic_index(&self, model: Option<String>) -> Self {
        self.index(FieldIndex::SemanticIndex {
            model,
            embedding_type: None,
        })
    }

    fn index(&self, index: FieldIndex) -> Self {
        Self {
            index: Some(index),
            ..self.clone()
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
                    DataType::F32Vector { dimension } => {
                        topk_protos::v1::control::field_type::DataType::F32Vector(
                            topk_protos::v1::control::FieldTypeF32Vector { dimension },
                        )
                    }
                    DataType::U8Vector { dimension } => {
                        topk_protos::v1::control::field_type::DataType::U8Vector(
                            topk_protos::v1::control::FieldTypeU8Vector { dimension },
                        )
                    }
                    DataType::BinaryVector { dimension } => {
                        topk_protos::v1::control::field_type::DataType::BinaryVector(
                            topk_protos::v1::control::FieldTypeBinaryVector { dimension },
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
        let env_env = Env::from_raw(env);

        let is_field_spec = {
            let env_value = Unknown::from_napi_value(env, value)?;
            FieldSpec::instance_of(env_env, env_value)?
        };

        if is_field_spec {
            FieldSpec::from_napi_value(env, value)
        } else {
            unreachable!("Value must be a FieldSpec")
        }
    }
}
