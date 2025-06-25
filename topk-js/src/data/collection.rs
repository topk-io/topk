use std::collections::HashMap;

use napi_derive::napi;

use crate::schema::{data_type::DataType, field_index::FieldIndexUnion};

#[napi(object)]
pub struct Collection {
    pub name: String,
    pub org_id: String,
    pub project_id: String,
    pub schema: HashMap<String, CollectionFieldSpec>,
    pub region: String,
}

impl From<topk_rs::proto::v1::control::Collection> for Collection {
    fn from(collection: topk_rs::proto::v1::control::Collection) -> Self {
        Self {
            name: collection.name,
            org_id: collection.org_id,
            project_id: collection.project_id,
            schema: collection
                .schema
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            region: collection.region,
        }
    }
}

#[napi(object)]
pub struct CollectionFieldSpec {
    #[napi(ts_type = "schema.DataType")]
    pub data_type: DataType,
    pub required: bool,
    #[napi(ts_type = "schema.FieldIndexUnion")]
    pub index: Option<FieldIndexUnion>,
}

impl From<topk_rs::proto::v1::control::FieldSpec> for CollectionFieldSpec {
    fn from(field_spec: topk_rs::proto::v1::control::FieldSpec) -> Self {
        Self {
            data_type: field_spec.data_type.unwrap().into(),
            required: field_spec.required,
            index: field_spec.index.map(|index| index.into()),
        }
    }
}
