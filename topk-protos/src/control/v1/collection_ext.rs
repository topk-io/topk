use std::collections::HashMap;

use super::*;

impl Collection {
    pub fn new(
        name: impl Into<String>,
        org_id: impl Into<String>,
        project_id: impl Into<String>,
        schema: impl Into<HashMap<String, FieldSpec>>,
        region: impl Into<String>,
    ) -> Self {
        Collection {
            name: name.into(),
            org_id: org_id.into(),
            project_id: project_id.into(),
            schema: schema.into(),
            region: region.into(),
        }
    }
}
