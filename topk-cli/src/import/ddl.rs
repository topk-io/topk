use std::collections::HashMap;

use topk_rs::proto::v1::control::{
    field_index::Index, field_type::DataType, field_type_matrix::MatrixValueType, FieldSpec,
    MultiVectorQuantization,
};

use crate::import::error::Error;
use crate::import::plan::{Field, Target};

pub type Schema = HashMap<String, FieldSpec>;

impl Target {
    /// The collection's schema was chosen at create: declared fields must match it, undeclared
    /// fields are written without one, and a declared required field must come from the source.
    pub fn validate_schema(&self, name: &str, got: &Schema) -> Result<(), Error> {
        let mut drift = Vec::new();
        for (field, want) in &self.fields {
            match got.get(field) {
                Some(got) if !want.matches(got) => drift.push(format!(
                    "{name}.{field}: want {}, the collection declares {}",
                    serde_json::to_string(&want.schema)?,
                    serde_json::to_string(got)?,
                )),
                None if want.schema.index.is_some() => drift.push(format!(
                    "{name}.{field}: an index needs the field declared, and the collection does not declare it"
                )),
                _ => {}
            }
        }
        for (field, spec) in got {
            if spec.required && !self.fields.contains_key(field) {
                drift.push(format!(
                    "{name}.{field}: the collection requires it, and this import leaves it out"
                ));
            }
        }
        if !drift.is_empty() {
            return Err(Error::SchemaMismatch(drift.join("\n  ")));
        }
        Ok(())
    }
}
impl Field {
    #[allow(deprecated)]
    pub fn matches(&self, got: &FieldSpec) -> bool {
        let want = &self.schema;
        if want.data_type != got.data_type || want.required != got.required {
            return false;
        }
        let mut index = want.index.clone();
        match (
            index.as_mut().and_then(|i| i.index.as_mut()),
            got.index.as_ref().and_then(|i| i.index.as_ref()),
        ) {
            (Some(Index::VectorIndex(want)), Some(Index::VectorIndex(got))) => {
                want.exact = want.exact.or(got.exact);
            }
            (Some(Index::SemanticIndex(want)), Some(Index::SemanticIndex(got))) => {
                want.model = want.model.clone().or_else(|| got.model.clone());
                want.embedding_type = want.embedding_type.or(got.embedding_type);
            }
            (Some(Index::MultiVectorIndex(index)), Some(Index::MultiVectorIndex(got))) => {
                index.encoding_version = got.encoding_version;
                if !index.skip_smve {
                    index.width.get_or_insert(32768);
                    index.top_k.get_or_insert(32);
                }
                if let Some(DataType::Matrix(matrix)) =
                    want.data_type.as_ref().and_then(|t| t.data_type.as_ref())
                {
                    if matches!(
                        matrix.value_type(),
                        MatrixValueType::F32 | MatrixValueType::F16 | MatrixValueType::F8
                    ) && index.quantization() == MultiVectorQuantization::Unspecified
                    {
                        index.quantization = Some(MultiVectorQuantization::Scalar.into());
                    }
                }
            }
            _ => {}
        }
        index == got.index
    }
}
#[cfg(test)]
mod tests {
    use super::Field;
    use rstest::rstest;
    use topk_rs::proto::v1::control::{
        field_index::Index, field_type_matrix::MatrixValueType, FieldIndex, FieldSpec, FieldType,
        MultiVectorDistanceMetric, MultiVectorQuantization, VectorDistanceMetric,
    };

    #[rstest]
    #[case::image_bytes(FieldType::image(), FieldType::bytes())]
    #[case::nested_types(
        FieldType::r#struct([("child", FieldSpec { data_type: Some(FieldType::integer()), ..Default::default() })]),
        FieldType::r#struct([("child", FieldSpec { data_type: Some(FieldType::text()), ..Default::default() })])
    )]
    fn preserves_types(#[case] want: FieldType, #[case] got: FieldType) {
        assert!(!Field::new(FieldSpec {
            data_type: Some(want),
            ..Default::default()
        })
        .matches(&FieldSpec {
            data_type: Some(got),
            ..Default::default()
        }));
    }

    #[test]
    fn semantic_defaults() {
        let want = FieldSpec::text(false).with_index(FieldIndex::semantic());
        let mut got = want.clone();
        if let Some(Index::SemanticIndex(index)) = got.index.as_mut().and_then(|i| i.index.as_mut())
        {
            index.model = Some("model-a".into());
            index.embedding_type = Some(1);
        }
        assert!(Field::new(want.clone()).matches(&got));
    }

    #[test]
    fn vector_defaults() {
        let want = FieldSpec {
            data_type: Some(FieldType::f32_vector(3)),
            index: Some(FieldIndex::vector(VectorDistanceMetric::Cosine)),
            required: false,
        };
        let mut got = want.clone();
        if let Some(Index::VectorIndex(index)) = got.index.as_mut().and_then(|i| i.index.as_mut()) {
            index.exact = Some(true);
        }
        assert!(Field::new(want.clone()).matches(&got));
        got.index = Some(FieldIndex::vector(VectorDistanceMetric::Euclidean));
        assert!(!Field::new(want.clone()).matches(&got));
    }

    #[rstest]
    #[case::float_default(MatrixValueType::F32, Some(MultiVectorQuantization::Scalar))]
    #[case::integer_default(MatrixValueType::U8, None)]
    fn matrix_defaults(
        #[case] ty: MatrixValueType,
        #[case] quant: Option<MultiVectorQuantization>,
    ) {
        let want = FieldSpec {
            data_type: Some(FieldType::matrix(128, ty)),
            index: Some(FieldIndex::multi_vector(
                MultiVectorDistanceMetric::Maxsim,
                None,
                None,
                None,
            )),
            required: false,
        };
        let mut got = want.clone();
        got.index = Some(FieldIndex::multi_vector(
            MultiVectorDistanceMetric::Maxsim,
            quant,
            Some(32768),
            Some(32),
        ));
        if let Some(Index::MultiVectorIndex(index)) =
            got.index.as_mut().and_then(|i| i.index.as_mut())
        {
            index.encoding_version = 2;
        }
        assert!(Field::new(want.clone()).matches(&got));
        if let Some(Index::MultiVectorIndex(index)) =
            got.index.as_mut().and_then(|i| i.index.as_mut())
        {
            index.width = Some(4096);
        }
        assert!(!Field::new(want.clone()).matches(&got));
    }
}
