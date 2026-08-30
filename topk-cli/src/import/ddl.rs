use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::Client;

use crate::import::error::Error;
use crate::import::spec::{inline, Field, Spec};

pub type Schema = HashMap<String, FieldSpec>;

/// Collections in the spec that do not exist yet; refuses on schema drift.
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
    .buffered(8)
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
        // An upsert replaces the document, and the drift error below tells people
        // to drop fields — so say what dropping them costs.
        let mut dropped: Vec<&str> = existing
            .schema
            .keys()
            .filter(|field| !schema.contains_key(*field))
            .map(String::as_str)
            .collect();
        if !dropped.is_empty() {
            dropped.sort_unstable();
            eprintln!(
                "# {name}: {} in the collection but not in this spec — re-imported rows lose them",
                dropped.join(", ")
            );
        }
        for (field, want) in schema.iter() {
            let want = Field::from(want);
            match existing.schema.get(field) {
                Some(got) if want == Field::from(got) => {}
                Some(got) => drift.push(format!(
                    "{name}.{field}: want {}, got {}",
                    inline(&want),
                    inline(&Field::from(got))
                )),
                None => drift.push(format!(
                    "{name}.{field}: want {}, field is missing",
                    inline(&want)
                )),
            }
        }
    }
    if !drift.is_empty() {
        return Err(Error::SchemaMismatch(drift.join("\n  ")));
    }
    Ok(absent)
}

pub async fn create(client: &Client, name: &str, schema: Schema) -> Result<(), Error> {
    match client.collections().create(name, schema, None).await {
        Ok(_) | Err(topk_rs::Error::CollectionAlreadyExists) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
