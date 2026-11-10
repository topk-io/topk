use indexmap::IndexMap;
use topk_rs::proto::v1::control::FieldSpec;
use wildmatch::WildMatch;

use crate::import::error::Error;
use crate::import::spec::{collection_key, Spec, Target};

#[derive(Clone)]
pub struct Table {
    pub from: String,
    pub collection_hint: Option<String>,
    pub columns: Vec<(String, Option<FieldSpec>)>,
    pub key: Option<String>,
}

pub struct Catalog {
    pub tables: Vec<Table>,
}

impl Catalog {
    pub fn table(&self, from: &str) -> Result<&Table, Error> {
        if let Some(table) = self.tables.iter().find(|table| table.from == from) {
            return Ok(table);
        }
        let mut matches = self
            .tables
            .iter()
            .filter(|table| !from.contains('.') && table.from.rsplit('.').next() == Some(from));
        match (matches.next(), matches.next()) {
            (Some(table), None) => Ok(table),
            (Some(_), Some(_)) => Err(Error::InvalidArgument(format!(
                "source object {from:?} is ambiguous; qualify it with its schema"
            ))),
            _ => Err(Error::InvalidArgument(format!(
                "source object {from:?} was not found"
            ))),
        }
    }

    pub fn discover(
        &self,
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

        let matched: Vec<&Table> = self
            .tables
            .iter()
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
            let key = match to {
                Some(to) => to.to_string(),
                None => {
                    let renamed = renames
                        .iter()
                        .find(|(from, _)| *from == object.from)
                        .map(|(_, name)| name.to_string());
                    match renamed
                        .or_else(|| object.collection_hint.as_deref().and_then(collection_key))
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

            let target = match Target::from_table(object, id) {
                Ok(target) => target,
                Err(Error::MissingId(from)) if match_count > 1 => {
                    tracing::warn!(source = %from, "skipping object: no id column found");
                    continue;
                }
                Err(error) => return Err(error),
            };
            // An id-only object must not sink a whole-database glob.
            if target.fields.is_empty() {
                tracing::warn!(source = %target.from, "skipping object: no columns to import besides the id");
                continue;
            }
            collections.insert(key, target);
        }

        if collections.is_empty() {
            return Err(Error::InvalidArgument(if match_count > 0 {
                format!("nothing to import: all {match_count} matched object(s) were skipped")
            } else if patterns.is_empty() {
                "the source has no objects to import".to_string()
            } else {
                let sample: Vec<&str> = self
                    .tables
                    .iter()
                    .take(5)
                    .map(|table| table.from.as_str())
                    .collect();
                format!(
                    "nothing to import from {patterns:?}. The source has {} object(s): {sample:?}",
                    self.tables.len()
                )
            }));
        }

        Spec::try_from(collections)
    }
}
