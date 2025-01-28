use std::collections::HashMap;

use super::*;

impl Collection {
    pub fn new(
        name: impl Into<String>,
        org_id: u64,
        project_id: u32,
        schema: impl Into<HashMap<String, FieldSpec>>,
    ) -> Self {
        Collection {
            name: name.into(),
            org_id,
            project_id,
            schema: schema.into(),
        }
    }
}
