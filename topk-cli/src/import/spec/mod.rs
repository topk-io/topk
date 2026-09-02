use std::str::FromStr;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use topk_rs::proto::v1::control::FieldSpec;

use crate::import::error::Error;
use crate::import::ID;

mod discover;
mod field;
mod render;
pub use discover::{discover, validate_columns};
pub use field::{Element, Field, Index, Type};
pub use render::{inline, render};

#[derive(Serialize)]
#[serde(transparent)]
pub struct Spec {
    pub collections: IndexMap<String, Target>,
}

#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub from: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub fields: IndexMap<String, Field>,
}

impl Target {
    /// The source column used as the document id.
    pub fn id_column(&self) -> &str {
        self.id.as_deref().unwrap_or(ID)
    }

    pub fn parsed_filter<F: FromStr<Err = Error>>(&self) -> Result<Option<F>, Error> {
        self.filter.as_deref().map(str::parse).transpose()
    }

    /// Every source column this target reads: the spec is a whitelist, so this
    /// is the whole projection.
    pub fn source_columns(&self) -> IndexSet<&str> {
        std::iter::once(self.id_column())
            .chain(self.fields.iter().map(|(name, field)| field.source(name)))
            .collect()
    }
}

fn collection_name(name: &str) -> Result<(), Error> {
    let mut chars = name.chars();
    let valid = matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        && name.len() <= 255;
    match valid {
        true => Ok(()),
        false => Err(Error::InvalidArgument(format!(
            "{name:?}: collection names start with a letter or digit, \
             then letters, digits, `_`, `.` or `-` (max 255 characters)"
        ))),
    }
}

/// Source object hint → collection name; None when no valid name derives.
pub fn collection_key(hint: &str) -> Option<String> {
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
        for (name, target) in collections.iter() {
            collection_name(name)?;
            if target.from.trim().is_empty() {
                return Err(Error::InvalidArgument(
                    "`from` is empty — name the table, index, collection or file path to read"
                        .to_string(),
                ));
            }
            // The spec is a whitelist, so no fields would import ids and nothing else.
            if target.fields.is_empty() {
                return Err(Error::InvalidArgument(format!(
                    "{name}: declare at least one field under [{name}.fields] — only \
                     declared fields are imported"
                )));
            }
            for (field_name, field) in &target.fields {
                if field_name.is_empty() || field_name.starts_with('_') {
                    return Err(Error::InvalidArgument(format!(
                        "{field_name:?}: field names cannot be empty or start with `_` — \
                         read the column under another name with `from = {field_name:?}`"
                    )));
                }
                FieldSpec::try_from(field)?;
            }
        }
        Ok(Spec { collections })
    }
}
