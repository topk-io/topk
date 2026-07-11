use topk_rs::proto::v1::control::{
    field_index, FieldSpec, KeywordIndexType, MultiVectorDistanceMetric, VectorDistanceMetric,
};

use super::Schema;
use crate::Error;

// A field's index, which decides how it is matched, scored, and sorted. The one
// place that cracks `spec.index`; every consumer reads this instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexKind {
    Keyword(KeywordIndexType),
    Semantic,
    Vector(VectorDistanceMetric),
    MultiVector(MultiVectorDistanceMetric),
    None,
}

impl From<&FieldSpec> for IndexKind {
    fn from(spec: &FieldSpec) -> IndexKind {
        match spec.index.as_ref().and_then(|i| i.index.as_ref()) {
            Some(field_index::Index::KeywordIndex(kw)) => IndexKind::Keyword(kw.index_type()),
            Some(field_index::Index::SemanticIndex(_)) => IndexKind::Semantic,
            Some(field_index::Index::VectorIndex(v)) => IndexKind::Vector(v.metric()),
            Some(field_index::Index::MultiVectorIndex(mv)) => IndexKind::MultiVector(mv.metric()),
            _ => IndexKind::None,
        }
    }
}

// Analyzed text and semantic fields have no exact per-document value, so ES
// rejects sort and aggregations over them (fielddata is disabled). Exact
// keyword, numeric, and boolean fields are fine.
pub fn ensure_aggregatable(schema: &Schema, field: &str) -> Result<(), Error> {
    match schema
        .get(field)
        .map(IndexKind::from)
        .unwrap_or(IndexKind::None)
    {
        IndexKind::Keyword(KeywordIndexType::Text) | IndexKind::Semantic => {
            Err(Error::Unsupported(format!(
                "Fielddata is disabled on text field [{field}]. Use a keyword field instead."
            )))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use topk_rs::proto::v1::control::{FieldIndex, FieldSpec};

    use super::*;

    #[test]
    fn classifies_index_by_kind() {
        let keyword =
            FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Exact));
        assert_eq!(
            IndexKind::from(&keyword),
            IndexKind::Keyword(KeywordIndexType::Exact)
        );

        let text = FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Text));
        assert_eq!(
            IndexKind::from(&text),
            IndexKind::Keyword(KeywordIndexType::Text)
        );

        let vector = FieldSpec::f32_vector(4, false)
            .with_index(FieldIndex::vector(VectorDistanceMetric::Cosine));
        assert_eq!(
            IndexKind::from(&vector),
            IndexKind::Vector(VectorDistanceMetric::Cosine)
        );

        // No index → None, not a spurious classification.
        assert_eq!(IndexKind::from(&FieldSpec::text(false)), IndexKind::None);
        assert_eq!(IndexKind::from(&FieldSpec::integer(false)), IndexKind::None);
    }
}
