use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::control::Dataset;

use crate::persistence;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatasetEntry {
    pub name: String,
    pub region: String,
}

impl From<Dataset> for DatasetEntry {
    fn from(dataset: Dataset) -> Self {
        Self {
            name: dataset.name,
            region: dataset.region,
        }
    }
}

impl From<DatasetEntry> for Dataset {
    fn from(entry: DatasetEntry) -> Self {
        Self {
            name: entry.name,
            region: entry.region,
            org_id: String::new(),
            project_id: String::new(),
            created_at: String::new(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DatasetIndex {
    #[serde(default)]
    pub datasets: Vec<DatasetEntry>,
}

impl DatasetIndex {
    /// Returns the entry for `name`, or `None` if not cached.
    ///
    /// Dataset names are unique per project (across all regions), so at most one
    /// entry can match.
    pub fn lookup(&self, name: &str) -> Option<&DatasetEntry> {
        self.datasets.iter().find(|d| d.name == name)
    }

    /// Replaces all entries from a full listing.
    pub fn set_all(&mut self, datasets: impl IntoIterator<Item = DatasetEntry>) {
        self.datasets = datasets.into_iter().collect();
    }

    /// Inserts an entry, replacing any existing one with the same `name`.
    pub fn insert(&mut self, entry: DatasetEntry) {
        self.datasets.retain(|d| d.name != entry.name);
        self.datasets.push(entry);
    }

    /// Evicts the cached entry for `name`.
    pub fn remove(&mut self, name: &str) {
        self.datasets.retain(|d| d.name != name);
    }
}

pub fn cache_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("topk").join("datasets.toml"))
}

pub(crate) fn load() -> DatasetIndex {
    persistence::load_toml_or_default(cache_path(), |path, err| {
        eprintln!(
            "warning: failed to parse file {}: {err}, defaulting to empty cache",
            path.display()
        );
    })
}

pub(crate) fn save(index: &DatasetIndex) -> Result<()> {
    persistence::save_toml(cache_path(), index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, region: &str) -> DatasetEntry {
        DatasetEntry {
            name: name.to_string(),
            region: region.to_string(),
        }
    }

    fn index(datasets: &[(&str, &str)]) -> DatasetIndex {
        let mut i = DatasetIndex::default();
        i.set_all(datasets.iter().map(|(n, r)| entry(n, r)));
        i
    }

    #[test]
    fn lookup_returns_entry_for_known_name() {
        let idx = index(&[("ds", "us-east-1")]);
        assert_eq!(
            idx.lookup("ds").map(|d| d.region.as_str()),
            Some("us-east-1")
        );
    }

    #[test]
    fn lookup_returns_none_for_unknown_name() {
        let idx = index(&[("other", "us-east-1")]);
        assert!(idx.lookup("missing").is_none());
    }

    #[test]
    fn set_all_replaces_existing_entries() {
        let mut idx = index(&[("old", "us-east-1")]);
        idx.set_all([entry("new", "eu-west-1")]);
        assert!(idx.lookup("new").is_some());
        assert!(idx.lookup("old").is_none());
    }

    #[test]
    fn insert_adds_entry_without_clearing_others() {
        let mut idx = index(&[("a", "us-east-1")]);
        idx.insert(entry("b", "eu-west-1"));
        assert!(idx.lookup("a").is_some());
        assert!(idx.lookup("b").is_some());
    }

    #[test]
    fn insert_replaces_same_name() {
        let mut idx = index(&[("a", "us-east-1")]);
        let updated = entry("a", "eu-west-1");
        idx.insert(updated);
        let entry = idx.lookup("a").unwrap();
        assert_eq!(entry.region, "eu-west-1");
    }

    #[test]
    fn remove_deletes_entry_for_name() {
        let mut idx = index(&[("a", "us-east-1"), ("b", "eu-west-1")]);
        idx.remove("a");
        assert!(idx.lookup("a").is_none());
        assert!(idx.lookup("b").is_some());
    }

    #[test]
    fn remove_evicts_entry_for_name() {
        let mut idx = index(&[("a", "us-east-1"), ("b", "eu-west-1")]);
        idx.remove("a");
        assert!(idx.lookup("a").is_none());
        assert!(idx.lookup("b").is_some());
    }

    #[test]
    fn serialization_roundtrip() {
        let idx = index(&[("ds1", "us-east-1"), ("ds2", "eu-west-1")]);
        let toml = toml::to_string_pretty(&idx).unwrap();
        let restored: DatasetIndex = toml::from_str(&toml).unwrap();
        assert!(restored.lookup("ds1").is_some());
        assert!(restored.lookup("ds2").is_some());
    }

    #[test]
    fn deserializes_legacy_format_with_fetched_at() {
        let legacy = r#"
fetched_at = "2026-01-01T00:00:00Z"

[[datasets]]
name = "ds"
region = "us-east-1"
"#;
        let restored: DatasetIndex = toml::from_str(legacy).unwrap();
        assert!(restored.lookup("ds").is_some());
    }
}
