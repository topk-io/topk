use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::import::error::Error;
use crate::import::source::Table;
use crate::import::ID;

mod field;
pub use field::Field;

#[derive(Clone, Serialize)]
#[serde(transparent)]
pub struct Spec {
    pub collections: IndexMap<String, Target>,
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub from: String,

    #[serde(default = "Target::default_id")]
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,

    pub fields: IndexMap<String, Field>,
}

impl Target {
    pub fn validate(&self) -> Result<(), Error> {
        if self.from.trim().is_empty() {
            return Err(Error::InvalidArgument(
                "`from` is empty — name the table, index, collection or file path to read".into(),
            ));
        }
        if self.id.trim().is_empty() {
            return Err(Error::InvalidArgument("`id` cannot be empty".into()));
        }
        // The spec is a whitelist, so no fields would import ids and nothing else.
        if self.fields.is_empty() {
            return Err(Error::InvalidArgument(
                "declare at least one field — only declared fields are imported".into(),
            ));
        }
        for (field_name, field) in &self.fields {
            field.validate()?;
            if field_name.is_empty() || field_name.starts_with('_') {
                return Err(Error::InvalidArgument(format!("{field_name:?}: field names cannot be empty or start with `_` — read the column under another name with `from = {field_name:?}`")));
            }
        }
        Ok(())
    }

    pub fn capped(&self, limit: u64) -> Self {
        Self {
            limit: Some(self.limit.map_or(limit, |existing| existing.min(limit))),
            ..self.clone()
        }
    }

    fn default_id() -> String {
        ID.to_string()
    }

    /// Every source column this target reads: the spec is a whitelist, so this
    /// is the whole projection.
    pub(super) fn source_columns(&self) -> IndexSet<&str> {
        std::iter::once(self.id.as_str())
            .chain(self.fields.iter().map(|(name, field)| field.source(name)))
            .chain(self.partition.as_deref())
            .collect()
    }
    pub fn from_table(table: &Table, id: Option<&str>) -> Result<Target, Error> {
        let id = id
            .or(table.key.as_deref())
            .or_else(|| {
                table
                    .columns
                    .iter()
                    .find(|(name, _)| matches!(name.to_ascii_lowercase().as_str(), "_id" | "id"))
                    .map(|(name, _)| name.as_str())
            })
            .ok_or_else(|| Error::MissingId(table.from.clone()))?;

        // Field names may not start with `_`, but sources use it for bookkeeping
        // columns (`_lang`, `_n_chars`); strip it and keep the source column in
        // `from`. A name that strips to nothing or onto another column is left
        // for validation to reject.
        let mut fields: IndexMap<String, Field> = IndexMap::new();
        let taken: HashSet<&str> = table
            .columns
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        for (name, field) in table
            .columns
            .iter()
            .filter(|(name, _)| name.as_str() != id)
            .filter_map(|(name, field)| field.as_ref().map(|field| (name, field)))
        {
            let stripped = name.trim_start_matches('_');
            if name.starts_with('_')
                && !stripped.is_empty()
                && !taken.contains(stripped)
                && !fields.contains_key(stripped)
            {
                let mut field = Field::new(field.clone());
                field.from = Some(name.clone());
                fields.insert(stripped.to_string(), field);
            } else {
                fields.insert(name.clone(), Field::new(field.clone()));
            }
        }

        Ok(Target {
            fields,
            from: table.from.clone(),
            id: id.to_string(),
            filter: None,
            partition: None,
            limit: None,
        })
    }
}

fn collection_name(name: &str) -> Result<(), Error> {
    let mut chars = name.chars();
    let valid = matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        && name.len() <= 255;
    match valid {
        true => Ok(()),
        false => Err(Error::InvalidArgument(format!("{name:?}: collection names start with a letter or digit, then letters, digits, `_`, `.` or `-` (max 255 characters)"))),
    }
}

/// Source object hint → collection name; None when no valid name derives.
pub(super) fn collection_key(hint: &str) -> Option<String> {
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
    collection_name(&key).ok().map(|()| key)
}

impl<'de> Deserialize<'de> for Spec {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Spec, D::Error> {
        IndexMap::<String, Target>::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

/// Every rule a spec must hold to, written by a user or derived by `discover`.
impl TryFrom<IndexMap<String, Target>> for Spec {
    type Error = Error;

    fn try_from(collections: IndexMap<String, Target>) -> Result<Spec, Error> {
        let spec = Spec { collections };
        spec.validate()?;
        Ok(spec)
    }
}

impl Spec {
    pub fn validate(&self) -> Result<(), Error> {
        for (name, target) in self.collections.iter() {
            collection_name(name)?;
            target.validate()?;
        }
        Ok(())
    }
}
