use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::{data_type::DataType, field_index::FieldIndex};

/// @internal
/// @hideconstructor
#[napi(namespace = "schema")]
#[derive(Clone, Debug)]
pub struct FieldSpec {
    data_type: DataType,
    required: bool,
    index: Option<FieldIndex>,
}

/// @ignore
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
    /// Marks the field as required. All fields are optional by default.
    ///
    /// Example:
    ///
    /// ```javascript
    /// import { text } from "topk-js/schema";
    ///
    /// await client.collections().create("books", {
    ///   title: text().required()
    /// });
    /// ```
    #[napi]
    pub fn required(&self) -> Self {
        Self {
            required: true,
            ..self.clone()
        }
    }

    /// Creates an index on a field.
    ///
    /// Example:
    ///
    /// ```javascript
    /// import { text, keywordIndex } from "topk-js/schema";
    ///
    /// await client.collections().create("books", {
    ///   title: text().index(keywordIndex())
    /// });
    /// ```
    #[napi]
    pub fn index(&self, index: FieldIndex) -> Self {
        Self {
            index: Some(index),
            ..self.clone()
        }
    }

    /// @ignore
    #[napi]
    pub fn to_string(&self) -> String {
        format!("{:?}", self)
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
                    DataType::F8Vector { dimension } => {
                        topk_rs::proto::v1::control::field_type::DataType::f8_vector(dimension)
                    }
                    DataType::F16Vector { dimension } => {
                        topk_rs::proto::v1::control::field_type::DataType::f16_vector(dimension)
                    }
                    DataType::F32Vector { dimension } => {
                        topk_rs::proto::v1::control::field_type::DataType::f32_vector(dimension)
                    }
                    DataType::U8Vector { dimension } => {
                        topk_rs::proto::v1::control::field_type::DataType::u8_vector(dimension)
                    }
                    DataType::I8Vector { dimension } => {
                        topk_rs::proto::v1::control::field_type::DataType::i8_vector(dimension)
                    }
                    DataType::BinaryVector { dimension } => {
                        topk_rs::proto::v1::control::field_type::DataType::binary_vector(dimension)
                    }
                    DataType::F32SparseVector => {
                        topk_rs::proto::v1::control::field_type::DataType::f32_sparse_vector()
                    }
                    DataType::F16SparseVector => {
                        topk_rs::proto::v1::control::field_type::DataType::f16_sparse_vector()
                    }
                    DataType::F8SparseVector => {
                        topk_rs::proto::v1::control::field_type::DataType::f8_sparse_vector()
                    }
                    DataType::I8SparseVector => {
                        topk_rs::proto::v1::control::field_type::DataType::i8_sparse_vector()
                    }
                    DataType::U8SparseVector => {
                        topk_rs::proto::v1::control::field_type::DataType::u8_sparse_vector()
                    }
                    DataType::Bytes => topk_rs::proto::v1::control::field_type::DataType::bytes(),
                    DataType::List { value_type } => {
                        topk_rs::proto::v1::control::field_type::DataType::List(value_type.into())
                    }
                    DataType::Struct { fields } => topk_rs::proto::v1::control::field_type::DataType::r#struct(
                        fields.into_iter().map(|(k, v)| (k, v.into())),
                    ),
                    DataType::Matrix {
                        dimension,
                        value_type,
                    } => topk_rs::proto::v1::control::field_type::DataType::matrix(
                        dimension,
                        value_type.into(),
                    ),
                }),
            }),
            required: field_spec.required,
            index: field_spec.index.map(|idx| idx.into()),
        }
    }
}

impl From<topk_rs::proto::v1::control::FieldSpec> for FieldSpec {
    fn from(proto: topk_rs::proto::v1::control::FieldSpec) -> Self {
        Self {
            data_type: proto
                .data_type
                .map(DataType::from)
                .expect("data_type is required"),
            required: proto.required,
            index: proto.index.map(|idx| idx.into()),
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

        let mut value_type: i32 = 0;
        check_status!(napi::sys::napi_typeof(env, value, &mut value_type))?;

        if value_type != napi::sys::ValueType::napi_object {
            return Err(napi::Error::from_reason(
                "Value must be a FieldSpec or plain object",
            ));
        }

        let mut is_array = false;
        check_status!(napi::sys::napi_is_array(env, value, &mut is_array))?;
        if is_array {
            return Err(napi::Error::from_reason("Array is not a valid field spec"));
        }

        let object = Object::from_napi_value(env, value)?;
        if let Ok(ctor) = object.get_named_property::<Unknown>("constructor") {
            let ctor_value = Unknown::to_napi_value(env, ctor)?;
            let mut ctor_type: i32 = 0;
            check_status!(napi::sys::napi_typeof(env, ctor_value, &mut ctor_type))?;
            if ctor_type == napi::sys::ValueType::napi_function {
                if let Ok(ctor_object) = Object::from_napi_value(env, ctor_value) {
                    if let Ok(name) = ctor_object.get_named_property::<String>("name") {
                        if name != "Object" {
                            return Err(napi::Error::from_reason(format!(
                                "Field spec must be a FieldSpec or plain object, got '{}' instance",
                                name
                            )));
                        }
                    }
                }
            }
        }

        let keys = Object::keys(&object)?;
        let mut fields = HashMap::new();
        for key in keys {
            let raw = object.get_named_property_unchecked::<Unknown>(&key)?;
            let raw_value = Unknown::to_napi_value(env, raw)?;
            fields.insert(key, FieldSpec::from_napi_value(env, raw_value)?);
        }

        Ok(FieldSpec::create(DataType::Struct { fields }))
    }
}
