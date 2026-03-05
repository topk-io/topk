use crate::error::TopkError;
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

impl TryFrom<topk_rs::proto::v1::control::Collection> for Collection {
    type Error = TopkError;

    fn try_from(
        collection: topk_rs::proto::v1::control::Collection,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name: collection.name,
            org_id: collection.org_id,
            project_id: collection.project_id,
            schema: collection
                .schema
                .into_iter()
                .map(|(k, v)| v.try_into().map(|spec| (k, spec)))
                .collect::<Result<HashMap<_, _>, _>>()?,
            region: collection.region,
        })
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

impl TryFrom<topk_rs::proto::v1::control::FieldSpec> for CollectionFieldSpec {
    type Error = TopkError;

    fn try_from(
        mut field_spec: topk_rs::proto::v1::control::FieldSpec,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            data_type: field_spec
                .data_type
                .take()
                .ok_or(topk_rs::Error::InvalidProto)?
                .try_into()?,
            required: field_spec.required,
            index: field_spec.index.map(|i| i.try_into()).transpose()?,
        })
    }
}
