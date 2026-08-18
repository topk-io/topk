use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};
use topk_rs::proto::v1::control::{field_index, field_type, FieldSpec, KeywordIndexType};
use topk_rs::Client;

use crate::import::error::Error;
use crate::import::spec::Spec;

pub type Schema = HashMap<String, FieldSpec>;

const CONCURRENCY: usize = 8;

/// Collections in the spec that do not exist yet, refusing on schema drift.
/// Runs before the confirmation prompt so a mismatch never surprises a `y`.
pub async fn absent(client: &Client, spec: &Spec) -> Result<HashMap<String, Schema>, Error> {
    let mut schemas: Vec<(&str, Schema)> = Vec::with_capacity(spec.collections.len());
    for (name, target) in spec.collections.iter() {
        let schema: Schema = target
            .fields
            .iter()
            .map(|(name, field)| Ok((name.clone(), FieldSpec::try_from(field)?)))
            .collect::<Result<_, Error>>()?;
        schemas.push((name, schema));
    }

    // `buffered` keeps spec order, so a drift report reads the same every run.
    let existing: Vec<_> = futures::stream::iter(schemas.iter().map(|(name, _)| async move {
        match client.collections().get(*name).await {
            Ok(collection) => Ok(Some(collection)),
            Err(topk_rs::Error::CollectionNotFound) => Ok(None),
            Err(e) => Err(Error::from(e)),
        }
    }))
    .buffered(CONCURRENCY)
    .try_collect()
    .await?;

    let mut drift = Vec::new();
    let mut absent = HashMap::new();
    for ((name, schema), existing) in schemas.into_iter().zip(existing) {
        let existing = match existing {
            Some(existing) => existing,
            None => {
                absent.insert(name.to_string(), schema);
                continue;
            }
        };
        for (field, want) in schema.iter() {
            match existing.schema.get(field) {
                Some(got)
                    if want.data_type == got.data_type
                        && index_field(want) == index_field(got) => {}
                Some(got) => drift.push(format!(
                    "{name}.{field}: want {}, got {}",
                    describe_field(want),
                    describe_field(got)
                )),
                None => drift.push(format!(
                    "{name}.{field}: want {}, field is missing",
                    describe_field(want)
                )),
            }
        }
    }
    if !drift.is_empty() {
        return Err(Error::SchemaMismatch(drift));
    }
    Ok(absent)
}

pub async fn create(client: &Client, name: &str, schema: Schema) -> Result<(), Error> {
    match client.collections().create(name, schema, None).await {
        Ok(_) | Err(topk_rs::Error::CollectionAlreadyExists) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn describe_field(spec: &FieldSpec) -> String {
    let ty = spec
        .data_type
        .as_ref()
        .and_then(|t| t.data_type.as_ref())
        .map_or("none", type_name);
    let ty = if spec.required {
        format!("required {ty}")
    } else {
        ty.to_string()
    };

    match index_field(spec) {
        Some(idx) => format!("{ty}, {idx} index"),
        None => ty,
    }
}

fn index_field(spec: &FieldSpec) -> Option<String> {
    Some(match spec.index.as_ref()?.index.as_ref()? {
        field_index::Index::KeywordIndex(k) => match KeywordIndexType::try_from(k.index_type) {
            Ok(KeywordIndexType::Exact) => "exact".to_string(),
            _ => "keyword".to_string(),
        },
        field_index::Index::SemanticIndex(_) => "semantic".to_string(),
        field_index::Index::VectorIndex(v) => format!("vector({})", v.metric),
        field_index::Index::MultiVectorIndex(_) => "maxsim".to_string(),
        field_index::Index::NgramIndex(_) => "ngram".to_string(),
    })
}

/// The spec vocabulary's name for a schema type, for drift messages.
fn type_name(dt: &field_type::DataType) -> &'static str {
    use field_type::DataType::*;
    match dt {
        Text(_) => "text",
        Integer(_) => "integer",
        Float(_) => "float",
        Boolean(_) => "boolean",
        Bytes(_) => "bytes",
        Timestamp(_) => "timestamp",
        Struct(_) => "struct",
        List(_) => "list",
        Matrix(_) => "matrix",
        F32Vector(_) => "f32_vector",
        F16Vector(_) => "f16_vector",
        F8Vector(_) => "f8_vector",
        U8Vector(_) => "u8_vector",
        I8Vector(_) => "i8_vector",
        BinaryVector(_) => "binary_vector",
        F32SparseVector(_) => "f32_sparse_vector",
        F16SparseVector(_) => "f16_sparse_vector",
        F8SparseVector(_) => "f8_sparse_vector",
        U8SparseVector(_) => "u8_sparse_vector",
        I8SparseVector(_) => "i8_sparse_vector",
    }
}
