use indexmap::IndexMap;
use wildmatch::WildMatch;

use crate::import::error::Error;
use crate::import::source::uri::Uri;
use crate::import::source::{self, Table};
use crate::import::spec::{invalid_collection_name, valid_collection_name, Field, Spec, Target};
use crate::import::ID_PLACEHOLDER;

pub async fn discover(
    uri: &Uri,
    patterns: &[String],
    to: Option<&str>,
    id: Option<&str>,
) -> Result<Spec, Error> {
    let available = source::connect(uri).await?.catalog(uri).await?;
    let total = available.len();
    let sample: Vec<String> = available.iter().take(5).map(|t| t.from.clone()).collect();

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

    let matched: Vec<Table> = available
        .into_iter()
        .filter(|object| {
            renames.iter().any(|(from, _)| *from == object.from)
                || globs.iter().any(|g| {
                    g.matches(&object.from)
                        || object
                            .collection_hint
                            .as_deref()
                            .and_then(collection_key)
                            .is_some_and(|key| g.matches(&key))
                })
        })
        .collect();

    if let Some(to) = to {
        if !valid_collection_name(to) {
            return Err(invalid_collection_name(to));
        }
        if matched.len() > 1 {
            return Err(Error::InvalidArgument(format!(
                "--to names a single collection, but {} objects matched — name them in a spec",
                matched.len()
            )));
        }
    }

    let match_count = matched.len();
    for mut object in matched {
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

        if !valid_collection_name(&key) {
            return Err(invalid_collection_name(&key));
        }
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
            eprintln!(
                "# skipping {}: no columns to import besides the id",
                target.from
            );
            skipped += 1;
            continue;
        }
        // A lone un-id-able match falls through so run() can point at --id.
        if target.id.as_deref() == Some(ID_PLACEHOLDER) && match_count > 1 {
            eprintln!(
                "# skipping {}: no id column found — import it alone with `--id <column>`, \
                 or set `id` in a spec",
                target.from
            );
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
            format!(
                "nothing to import from {patterns:?}. The source has {total} object(s): {sample:?}"
            )
        }));
    }

    let spec = Spec { collections };
    spec.validate()?;
    Ok(spec)
}

// Source object hint → TopK collection name; None when no valid name derives.
fn collection_key(hint: &str) -> Option<String> {
    let key: String = hint
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    valid_collection_name(&key).then_some(key)
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
        let taken: std::collections::HashSet<&str> = table
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
