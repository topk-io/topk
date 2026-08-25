use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use topk_rs::proto::v1::control::FieldSpec;

use crate::import::error::Error;

mod field;
pub use field::{Field, Index, Quant, Type};

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct Spec {
    pub collections: IndexMap<String, Target>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
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

pub fn valid_collection_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        && name.len() <= 255
}

pub fn invalid_collection_name(name: &str) -> Error {
    Error::InvalidArgument(format!(
        "{name:?}: collection names start with a letter or digit, \
         then letters, digits, `_`, `.` or `-` (max 255 characters)"
    ))
}

impl Spec {
    pub fn parse(s: &str) -> Result<Spec, Error> {
        let spec: Spec = toml::from_str(s)?;
        spec.validate()?;
        Ok(spec)
    }

    /// Every rule a spec must hold to, written by a user or derived by `discover`.
    pub fn validate(&self) -> Result<(), Error> {
        for (name, target) in self.collections.iter() {
            if !valid_collection_name(name) {
                return Err(invalid_collection_name(name));
            }
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
                        "{field_name:?}: field names cannot be empty or start with `_`"
                    )));
                }
                FieldSpec::try_from(field)?;
            }
        }
        Ok(())
    }
}
