use crate::schema::{data_type::DataType, field_index::FieldIndexUnion};
use napi_derive::napi;
use std::collections::HashMap;

/// Represents a collection in the TopK service.
///
/// A collection is a container for documents with a defined schema.
/// This struct contains metadata about the collection including its name,
/// organization, project, schema, and region.
#[napi(object)]
pub struct Collection {
    /// Name of the collection
    pub name: String,
    /// Organization ID that owns the collection
    pub org_id: String,
    /// Project ID that contains the collection
    pub project_id: String,
    /// Schema definition for the collection fields
    pub schema: HashMap<String, CollectionFieldSpec>,
    /// Region where the collection is stored
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

/// Represents a field specification within a collection schema.
///
/// This struct defines the properties of a field in a collection,
/// including its data type, whether it's required, and any index configuration.
#[napi(object)]
pub struct CollectionFieldSpec {
    /// Data type of the field
    #[napi(ts_type = "schema.DataType")]
    pub data_type: DataType,
    /// Whether the field is required (must be present in all documents)
    pub required: bool,
    /// Index configuration for the field (optional)
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
