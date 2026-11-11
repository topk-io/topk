use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};
use topk_rs::proto::v1::control::{
    field_index::Index, field_type::DataType, field_type_matrix::MatrixValueType, FieldSpec,
    MultiVectorQuantization,
};
use topk_rs::Client;

use crate::import::error::Error;
use crate::import::spec::Target;

pub type Schema = HashMap<String, FieldSpec>;

/// Collections in the spec that do not exist yet; refuses on schema drift.
pub async fn absent<'a>(
    client: &Client,
    targets: impl Iterator<Item = (&'a String, &'a Target)>,
) -> Result<HashMap<String, Schema>, Error> {
    // `buffered` keeps spec order, so a drift report reads the same every run.
    let existing: Vec<_> = futures::stream::iter(targets.map(|(name, target)| async move {
        match client.collections().get(name).await {
            Ok(collection) => Ok((name, target, Some(collection))),
            Err(topk_rs::Error::CollectionNotFound) => Ok((name, target, None)),
            Err(e) => Err(Error::from(e)),
        }
    }))
    .buffered(8)
    .try_collect()
    .await?;

    let mut drift = Vec::new();
    let mut absent = HashMap::new();
    for (name, target, existing) in existing {
        let existing = match existing {
            Some(existing) => existing,
            None => {
                absent.insert(
                    name.clone(),
                    target
                        .fields
                        .iter()
                        .map(|(name, field)| (name.clone(), field.schema.clone()))
                        .collect(),
                );
                continue;
            }
        };
        for (field, want) in &target.fields {
            let want = &want.schema;
            match existing.schema.get(field) {
                Some(got) => {
                    if !matches(want, got) {
                        drift.push(format!(
                            "{name}.{field}: want {}, got {}",
                            serde_json::to_string(want)?,
                            serde_json::to_string(got)?
                        ));
                    }
                }
                None => drift.push(format!(
                    "{name}.{field}: want {}, field is missing",
                    serde_json::to_string(want)?
                )),
            }
        }
    }
    if !drift.is_empty() {
        return Err(Error::SchemaMismatch(drift.join("\n  ")));
    }
    Ok(absent)
}

#[allow(deprecated)]
fn matches(want: &FieldSpec, got: &FieldSpec) -> bool {
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

#[cfg(test)]
mod tests {
    use super::matches;
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
        assert!(!matches(
            &FieldSpec {
                data_type: Some(want),
                ..Default::default()
            },
            &FieldSpec {
                data_type: Some(got),
                ..Default::default()
            },
        ));
    }

    #[rstest]
    #[case::explicit_exact(
        "f32_vector",
        "dim = 3",
        "vector = { metric = 'cosine', exact = false }",
        "vector = { metric = 'cosine', exact = true }"
    )]
    #[case::semantic_model(
        "text",
        "",
        "semantic = { model = 'model-a' }",
        "semantic = { model = 'model-b' }"
    )]
    #[case::semantic_embedding(
        "text",
        "",
        "semantic = { embedding_type = 1 }",
        "semantic = { embedding_type = 2 }"
    )]
    fn explicit_index_drift(
        #[case] ty: &str,
        #[case] dimensions: &str,
        #[case] want: &str,
        #[case] got: &str,
    ) {
        let want: crate::import::spec::Field = toml::from_str(&format!(
            "type = '{ty}'\nindex = {{ {want} }}\n{}",
            dimensions
        ))
        .unwrap();
        let got: crate::import::spec::Field = toml::from_str(&format!(
            "type = '{ty}'\nindex = {{ {got} }}\n{}",
            dimensions
        ))
        .unwrap();
        assert!(!matches(&want.schema, &got.schema));
    }

    #[test]
    fn semantic_defaults() {
        let want: crate::import::spec::Field =
            toml::from_str("type = 'text'\nindex = 'semantic'").unwrap();
        let got: crate::import::spec::Field = toml::from_str(
            "type = 'text'\nindex = { semantic = { model = 'model-a', embedding_type = 1 } }",
        )
        .unwrap();
        assert!(matches(&want.schema, &got.schema));
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
        assert!(matches(&want, &got));
        got.index = Some(FieldIndex::vector(VectorDistanceMetric::Euclidean));
        assert!(!matches(&want, &got));
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
        assert!(matches(&want, &got));
        if let Some(Index::MultiVectorIndex(index)) =
            got.index.as_mut().and_then(|i| i.index.as_mut())
        {
            index.width = Some(4096);
        }
        assert!(!matches(&want, &got));
    }
}
