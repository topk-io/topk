use std::collections::HashMap;

use serde::Serialize;

use super::{FieldMapping, IndexName, MappingProperties};

#[derive(Serialize)]
pub struct FieldCapsBody {
    pub indices: Vec<IndexName>,
    pub fields: HashMap<String, HashMap<&'static str, FieldCap>>,
}

#[derive(Serialize)]
pub struct FieldCap {
    #[serde(rename = "type")]
    pub field_type: &'static str,
    pub metadata_field: bool,
    pub searchable: bool,
    pub aggregatable: bool,
    pub inference: bool,
}

impl From<&FieldMapping> for FieldCap {
    fn from(mapping: &FieldMapping) -> Self {
        let indexed = |index: &Option<bool>| index.unwrap_or(true);

        let (field_type, searchable, aggregatable) = match mapping {
            FieldMapping::Text { index, .. } => ("text", indexed(index), false),
            FieldMapping::Keyword { index, .. } => ("keyword", indexed(index), true),
            FieldMapping::Integer { .. } => ("integer", true, true),
            FieldMapping::Date { .. } => ("date", true, true),
            FieldMapping::Float { .. } => ("float", true, true),
            FieldMapping::Boolean { .. } => ("boolean", true, true),
            FieldMapping::Object { .. } => ("object", false, false),
            FieldMapping::DenseVector { index, .. } => ("dense_vector", indexed(index), false),
            // ES reports rank_vectors unsearchable because it can only rerank with them; ours
            // carry a multi_vector index and are queryable.
            FieldMapping::RankVectors { index, .. } => ("rank_vectors", indexed(index), false),
            FieldMapping::SemanticText { .. } => ("semantic_text", true, false),
        };

        Self {
            field_type,
            metadata_field: false,
            searchable,
            aggregatable,
            inference: false,
        }
    }
}

impl FieldCapsBody {
    pub fn new(index: IndexName, properties: MappingProperties, fields: &[String]) -> Self {
        let all = fields.is_empty() || fields.iter().any(|f| f == "*");

        let fields = properties
            .0
            .iter()
            .filter(|(name, _)| all || fields.iter().any(|f| f == *name))
            .map(|(name, mapping)| {
                let cap = FieldCap::from(mapping);
                (name.clone(), HashMap::from([(cap.field_type, cap)]))
            })
            .collect();

        Self {
            indices: vec![index],
            fields,
        }
    }
}
