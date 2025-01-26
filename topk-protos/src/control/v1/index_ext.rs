use super::*;

impl Index {
    pub fn new(
        name: impl Into<String>,
        org_id: u64,
        project_id: u32,
        schema: index_schema::IndexSchema,
    ) -> Self {
        Index {
            name: name.into(),
            org_id,
            project_id,
            schema: schema.into_fields(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexName(pub String);

impl From<String> for IndexName {
    fn from(name: String) -> Self {
        IndexName(name)
    }
}

impl From<IndexName> for String {
    fn from(name: IndexName) -> Self {
        name.0
    }
}
