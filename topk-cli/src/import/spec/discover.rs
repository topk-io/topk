use std::collections::HashSet;

use indexmap::IndexMap;
use wildmatch::WildMatch;

use crate::import::error::Error;
use crate::import::source::Table;
use crate::import::spec::{collection_key, Field, Spec, Target};
use crate::import::{ID, ID_PLACEHOLDER};

/// Bind every field to the source column it reads: the column has to be one the
/// catalog lists, and a field that declares no `type` takes the column's. Caught
/// before any cluster write, for a spec however it was assembled: discovered,
/// `-f`, or resumed. A sampled catalog (mongodb) lists nothing to bind against,
/// where an absent column is no proof the source lacks it — there a field has to
/// declare its own type.
pub fn bind_columns(catalog: &[Table], spec: Spec) -> Result<Spec, Error> {
    let mut collections = spec.collections;
    for (name, target) in collections.iter_mut() {
        let from = target.from.clone();
        let table = catalog.iter().find(|table| table.from == from);
        let column = |wanted: &str| {
            table.and_then(|table| table.columns.iter().find(|(name, _)| name == wanted))
        };
        let absent = |what: String| {
            let available: Vec<&str> = table
                .map(|table| table.columns.iter().map(|(c, _)| c.as_str()).collect())
                .unwrap_or_default();
            Error::InvalidArgument(format!(
                "{name}: {what} is not in {from:?} — available: {}",
                available.join(", ")
            ))
        };
        // A placeholder id is "not detected", tolerated so --dry-run can show it.
        let id = target.id_column();
        if table.is_some() && id != ID_PLACEHOLDER && column(id).is_none() {
            return Err(absent(format!("id column {id:?}")));
        }
        for (field_name, field) in target.fields.iter_mut() {
            // The key's column is checked above, and its type is never read.
            if field_name == ID {
                continue;
            }
            let source = field.source(field_name).to_string();
            let found = column(&source);
            if table.is_some() && found.is_none() {
                return Err(absent(format!(
                    "field {field_name:?} reads column {source:?}, which"
                )));
            }
            if field.ty.is_none() {
                field.ty = found.and_then(|(_, source)| source.ty);
                // A width discovered with the type, or a vector arrives without one.
                field.dim = field
                    .dim
                    .or_else(|| found.and_then(|(_, source)| source.dim));
            }
            if field.ty.is_none() {
                return Err(Error::InvalidArgument(format!(
                    "{name}.{field_name}: declare `type` — {from:?} reports none for \
                     column {source:?}"
                )));
            }
        }
    }
    // Back through the one door, now that every field has a type.
    Spec::try_from(collections)
}

pub fn discover(
    available: &[Table],
    patterns: &[String],
    to: Option<&str>,
    id: Option<&str>,
) -> Result<Spec, Error> {
    let mut renames: Vec<(&str, &str)> = Vec::new();
    let mut globs: Vec<WildMatch> = Vec::new();
    for pattern in patterns {
        match pattern.split_once('=') {
            Some((object, name)) => renames.push((object, name)),
            None => globs.push(WildMatch::new(pattern)),
        }
    }
    if patterns.is_empty() {
        globs.push(WildMatch::new("*"));
    }
    let mut collections: IndexMap<String, Target> = IndexMap::new();
    let mut skipped = 0;

    // Partitioned, not filtered: what did not match is the sample an empty
    // result reports, without cloning it on every run that succeeds.
    let (matched, rest): (Vec<&Table>, Vec<&Table>) = available.iter().partition(|object| {
        renames.iter().any(|(from, _)| *from == object.from)
            || globs.iter().any(|g| {
                g.matches(&object.from)
                    || object
                        .collection_hint
                        .as_deref()
                        .and_then(collection_key)
                        .is_some_and(|key| g.matches(&key))
            })
    });

    if to.is_some() && matched.len() > 1 {
        return Err(Error::InvalidArgument(format!(
            "--to names a single collection, but {} objects matched — name them in a spec",
            matched.len()
        )));
    }
    if id.is_some() && matched.len() > 1 {
        return Err(Error::InvalidArgument(format!(
            "--id names the id column of a single object, but {} objects matched — \
             set `id` per collection in a spec",
            matched.len()
        )));
    }

    let match_count = matched.len();
    for object in matched {
        let mut object = object.clone();
        // Via the primary key, so Target::from also drops the column from the fields.
        if let Some(id) = id {
            object.primary_key = Some(id.to_string());
        }
        let key = match to {
            Some(to) => to.to_string(),
            None => {
                let renamed = renames
                    .iter()
                    .find(|(from, _)| *from == object.from)
                    .map(|(_, name)| name.to_string());
                match renamed.or_else(|| object.collection_hint.as_deref().and_then(collection_key))
                {
                    Some(key) => key,
                    None => {
                        return Err(Error::InvalidArgument(format!(
                            "cannot derive a collection name from {:?} — pass --to <name>",
                            object.from
                        )))
                    }
                }
            }
        };

        if let Some(existing) = collections.get(&key) {
            return Err(Error::InvalidArgument(format!(
                "{:?} and {:?} both map to collection {key:?}: rename one inline, \
                 e.g. '{}=<name>'",
                existing.from, object.from, object.from
            )));
        }

        let target = Target::from(object);
        // An id-only object must not sink a whole-database glob.
        if target.declared().next().is_none() {
            crate::import::note(format!(
                "skipping {}: no columns to import besides the id",
                target.from
            ));
            skipped += 1;
            continue;
        }
        // A lone un-id-able match falls through so run() can point at --id.
        if target.id.as_deref() == Some(ID_PLACEHOLDER) && match_count > 1 {
            crate::import::note(format!(
                "skipping {}: no id column found — import it alone with `--id <column>`, \
                 or set `id` in a spec",
                target.from
            ));
            skipped += 1;
            continue;
        }
        collections.insert(key, target);
    }

    if collections.is_empty() {
        return Err(Error::InvalidArgument(if skipped > 0 {
            format!("nothing to import: all {skipped} matched object(s) were skipped")
        } else if patterns.is_empty() {
            "the source has no objects to import".to_string()
        } else {
            let sample: Vec<&str> = rest
                .iter()
                .take(5)
                .map(|table| table.from.as_str())
                .collect();
            format!(
                "nothing to import from {patterns:?}. The source has {} object(s): {sample:?}",
                rest.len()
            )
        }));
    }

    Spec::try_from(collections)
}

impl From<Table> for Target {
    fn from(table: Table) -> Target {
        let id = table
            .primary_key
            .or_else(|| {
                table
                    .columns
                    .iter()
                    .find(|(name, _)| matches!(name.to_ascii_lowercase().as_str(), "_id" | "id"))
                    .map(|(name, _)| name.clone())
            })
            .unwrap_or_else(|| ID_PLACEHOLDER.to_string());

        // Field names may not start with `_`, but sources use it for bookkeeping
        // columns (`_lang`, `_n_chars`); strip it and keep the source column in
        // `from`. A name that strips to nothing or onto another column is left
        // for validation to reject.
        let mut fields: indexmap::IndexMap<String, Field> = indexmap::IndexMap::new();
        let taken: HashSet<&str> = table
            .columns
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        for (name, field) in table.columns.iter().filter(|(name, _)| *name != id) {
            let stripped = name.trim_start_matches('_');
            if name.starts_with('_')
                && !stripped.is_empty()
                && !taken.contains(stripped)
                && !fields.contains_key(stripped)
            {
                let mut field = field.clone();
                field.from = Some(name.clone());
                fields.insert(stripped.to_string(), field);
            } else {
                fields.insert(name.clone(), field.clone());
            }
        }

        // The key comes first, as the field it is.
        fields.shift_insert(
            0,
            ID.to_string(),
            Field {
                from: Some(id),
                ..Default::default()
            },
        );
        Target {
            fields,
            from: table.from,
            ..Default::default()
        }
    }
}
