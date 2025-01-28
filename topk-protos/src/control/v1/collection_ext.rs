use super::*;

impl Collection {
    pub fn new(
        name: impl Into<String>,
        org_id: u64,
        project_id: u32,
        schema: collection_schema::CollectionSchema,
    ) -> Self {
        Collection {
            name: name.into(),
            org_id,
            project_id,
            schema: schema.into_fields(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CollectionName(pub String);

impl From<String> for CollectionName {
    fn from(name: String) -> Self {
        CollectionName(name)
    }
}

impl From<CollectionName> for String {
    fn from(name: CollectionName) -> Self {
        name.0
    }
}
