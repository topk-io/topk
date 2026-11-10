use std::num::NonZeroU64;
use std::path::PathBuf;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use topk_rs::proto::v1::control::FieldSpec;

use crate::import::error::Error;
use crate::import::{Source, ID};

mod discover;
mod field;
mod render;
pub use discover::discover;
pub use field::{Element, Field, Index, Type};
pub use render::inline;

#[derive(clap::Args, Debug)]
pub struct PlanOptions {
    #[arg(
        conflicts_with = "spec",
        help = "Source objects to import; exact names, globs, or <object>=<collection>"
    )]
    pub objects: Vec<String>,

    #[arg(
        short = 'f',
        long,
        value_name = "SPEC",
        help = "Read a TOML import spec"
    )]
    pub spec: Option<PathBuf>,
    #[arg(
        long,
        value_name = "COLLECTION",
        conflicts_with = "spec",
        help = "Name the target collection; required when it cannot be derived"
    )]
    pub to: Option<String>,
    #[arg(
        long,
        value_name = "COLUMN",
        conflicts_with = "spec",
        help = "Column to use as the document id (_id); use when it can't be auto-detected"
    )]
    pub id: Option<String>,
    #[arg(
        long,
        value_name = "COLUMN",
        help = "Column whose value is the partition; each row goes to its own"
    )]
    pub partition: Option<String>,
    #[arg(
        conflicts_with = "spec",
        long,
        help = "Only read rows matching a filter, in the source's language: \
                SQL WHERE for files and databases, a JSON query for mongodb/elasticsearch"
    )]
    pub filter: Option<String>,
    // Broadcasts to every collection like --partition, so it is allowed with a spec.
    #[arg(long, help = "Read at most this many rows per object")]
    pub limit: Option<NonZeroU64>,
}

impl PlanOptions {
    pub async fn build(&self, source: &Source) -> Result<Spec, Error> {
        let (mut spec, catalog) = match &self.spec {
            Some(path) => (
                toml::from_str::<Spec>(&std::fs::read_to_string(path).map_err(|error| {
                    Error::Setup(
                        anyhow::Error::new(error)
                            .context(format!("cannot read spec {}", path.display())),
                    )
                })?)?,
                None,
            ),
            None => {
                let catalog = source.catalog().await?;
                let spec = discover(
                    &catalog,
                    &self.objects,
                    self.to.as_deref(),
                    self.id.as_deref(),
                )?;
                (spec, Some(catalog))
            }
        };
        if self.filter.is_some() && spec.collections.len() > 1 {
            return Err(Error::InvalidArgument(format!(
                "--filter applies to a single object, but {} objects matched — set `filter` per collection in a spec",
                spec.collections.len()
            )));
        }
        for target in spec.collections.values_mut() {
            if let Some(filter) = &self.filter {
                target.filter = Some(filter.clone());
            }
            if let Some(limit) = self.limit {
                target.limit = Some(limit.get());
            }
            if let Some(partition) = &self.partition {
                target.partition = Some(partition.clone());
            }
        }
        source
            .validate(spec.collections.iter(), catalog.as_deref())
            .await?;
        Ok(spec)
    }
}

#[derive(Clone, Serialize)]
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

    /// Every source column this target reads: the spec is a whitelist, so this
    /// is the whole projection.
    pub fn source_columns(&self) -> IndexSet<&str> {
        std::iter::once(self.id_column())
            .chain(self.fields.iter().map(|(name, field)| field.source(name)))
            .chain(self.partition.as_deref())
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
        false => Err(Error::InvalidArgument(format!("{name:?}: collection names start with a letter or digit, then letters, digits, `_`, `.` or `-` (max 255 characters)"))),
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
            if target.limit == Some(0) {
                return Err(Error::InvalidArgument(format!(
                    "{name}: limit must be greater than zero"
                )));
            }
            if target.from.trim().is_empty() {
                return Err(Error::InvalidArgument(
                    "`from` is empty — name the table, index, collection or file path to read"
                        .into(),
                ));
            }
            // The spec is a whitelist, so no fields would import ids and nothing else.
            if target.fields.is_empty() {
                return Err(Error::InvalidArgument(format!("{name}: declare at least one field under [{name}.fields] — only declared fields are imported")));
            }
            for (field_name, field) in &target.fields {
                if field_name.is_empty() || field_name.starts_with('_') {
                    return Err(Error::InvalidArgument(format!("{field_name:?}: field names cannot be empty or start with `_` — read the column under another name with `from = {field_name:?}`")));
                }
                FieldSpec::try_from(field)?;
            }
        }
        Ok(Spec { collections })
    }
}
