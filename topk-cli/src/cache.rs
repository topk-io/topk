use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use topk_rs::{proto::v1::control::Dataset, Error};

use crate::config::{load_toml_or_default, save_toml};

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
struct DatasetIndex {
    #[serde(default)]
    datasets: Vec<DatasetEntry>,
}

impl DatasetIndex {
    fn lookup(&self, name: &str) -> Option<&DatasetEntry> {
        self.datasets.iter().find(|d| d.name == name)
    }

    fn set_all(&mut self, datasets: impl IntoIterator<Item = DatasetEntry>) {
        self.datasets = datasets.into_iter().collect();
    }

    fn insert(&mut self, entry: DatasetEntry) {
        self.datasets.retain(|d| d.name != entry.name);
        self.datasets.push(entry);
    }

    fn remove(&mut self, name: &str) {
        self.datasets.retain(|d| d.name != name);
    }
}

pub struct Cache {
    path: Option<PathBuf>,
    index: DatasetIndex,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            path: None,
            index: DatasetIndex::default(),
        }
    }
}

impl Cache {
    pub fn new(path: Option<PathBuf>) -> Self {
        let index = load_toml_or_default(path.clone(), |p, err| {
            eprintln!(
                "warning: failed to parse file {}: {err}, defaulting to empty cache",
                p.display()
            );
        });
        Self { path, index }
    }

    pub fn save(&self) -> Result<(), Error> {
        save_toml(self.path.clone(), &self.index)
    }

    pub fn lookup(&self, name: &str) -> Option<&DatasetEntry> {
        self.index.lookup(name)
    }

    pub fn set_all(&mut self, datasets: impl IntoIterator<Item = DatasetEntry>) {
        self.index.set_all(datasets);
    }

    pub fn insert(&mut self, entry: DatasetEntry) {
        self.index.insert(entry);
    }

    pub fn remove(&mut self, name: &str) {
        self.index.remove(name);
    }
}

pub fn cache_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("topk").join("datasets.toml"))
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
        let idx = index(&[("ds", "aws-us-east-1-elastica")]);
        assert_eq!(
            idx.lookup("ds").map(|d| d.region.as_str()),
            Some("aws-us-east-1-elastica")
        );
    }

    #[test]
    fn lookup_returns_none_for_unknown_name() {
        let idx = index(&[("other", "aws-us-east-1-elastica")]);
        assert!(idx.lookup("missing").is_none());
    }

    #[test]
    fn set_all_replaces_existing_entries() {
        let mut idx = index(&[("old", "aws-us-east-1-elastica")]);
        idx.set_all([entry("new", "aws-eu-central-1-monstera")]);
        assert!(idx.lookup("new").is_some());
        assert!(idx.lookup("old").is_none());
    }

    #[test]
    fn insert_adds_entry_without_clearing_others() {
        let mut idx = index(&[("a", "aws-us-east-1-elastica")]);
        idx.insert(entry("b", "aws-eu-central-1-monstera"));
        assert!(idx.lookup("a").is_some());
        assert!(idx.lookup("b").is_some());
    }

    #[test]
    fn insert_replaces_same_name() {
        let mut idx = index(&[("a", "aws-us-east-1-elastica")]);
        let updated = entry("a", "aws-eu-central-1-monstera");
        idx.insert(updated);
        let entry = idx.lookup("a").unwrap();
        assert_eq!(entry.region, "aws-eu-central-1-monstera");
    }

    #[test]
    fn remove_deletes_entry_for_name() {
        let mut idx = index(&[
            ("a", "aws-us-east-1-elastica"),
            ("b", "aws-eu-central-1-monstera"),
        ]);
        idx.remove("a");
        assert!(idx.lookup("a").is_none());
        assert!(idx.lookup("b").is_some());
    }

    #[test]
    fn serialization_roundtrip() {
        let idx = index(&[
            ("dataset-1", "aws-us-east-1-elastica"),
            ("dataset-2", "aws-eu-central-1-monstera"),
        ]);
        let toml = toml::to_string_pretty(&idx).unwrap();
        let restored: DatasetIndex = toml::from_str(&toml).unwrap();
        assert!(restored.lookup("dataset-1").is_some());
        assert!(restored.lookup("dataset-2").is_some());
    }
}
