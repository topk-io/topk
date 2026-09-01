use std::collections::HashSet;

use indexmap::IndexMap;
use wildmatch::WildMatch;

use crate::import::error::Error;
use crate::import::source::Table;
use crate::import::spec::{collection_key, Field, Spec, Target};
use crate::import::ID_PLACEHOLDER;

/// Every target reads the columns `SELECT *` yields, so a declared id that the
/// catalog does not list would sink the whole run one row in — after the
/// collection is created. Caught here, before any cluster write, for a spec
/// however it was assembled: discovered, `-f`, or resumed.
pub fn validate_ids(catalog: &[Table], spec: &Spec) -> Result<(), Error> {
    for (name, target) in spec.collections.iter() {
        let id = match target.id.as_deref() {
            // A placeholder is "not detected", tolerated so --dry-run can show it.
            Some(id) if id != ID_PLACEHOLDER => id,
            _ => continue,
        };
        let Some(table) = catalog.iter().find(|table| table.from == target.from) else {
            continue;
        };
        if !table.columns.iter().any(|(column, _)| column == id) {
            let available: Vec<&str> = table.columns.iter().map(|(c, _)| c.as_str()).collect();
            return Err(Error::InvalidArgument(format!(
                "{name}: id column {id:?} is not in {:?} — available: {}",
                target.from,
                available.join(", ")
            )));
        }
    }
    Ok(())
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
        if target.fields.is_empty() {
            crate::import::note(format!(
                "# skipping {}: no columns to import besides the id",
                target.from
            ));
            skipped += 1;
            continue;
        }
        // A lone un-id-able match falls through so run() can point at --id.
        if target.id.as_deref() == Some(ID_PLACEHOLDER) && match_count > 1 {
            crate::import::note(format!(
                "# skipping {}: no id column found — import it alone with `--id <column>`, \
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

        Target {
            fields,
            from: table.from,
            id: Some(id),
            ..Default::default()
        }
    }
}
