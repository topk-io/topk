use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::control::Dataset;

const TTL_MINUTES: i64 = 5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatasetEntry {
    pub name: String,
    pub region: String,
    #[serde(default)]
    pub org_id: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub created_at: String,
}

impl From<Dataset> for DatasetEntry {
    fn from(dataset: Dataset) -> Self {
        Self {
            name: dataset.name,
            region: dataset.region,
            org_id: dataset.org_id,
            project_id: dataset.project_id,
            created_at: dataset.created_at,
        }
    }
}

impl From<DatasetEntry> for Dataset {
    fn from(dataset: DatasetEntry) -> Self {
        Self {
            name: dataset.name,
            region: dataset.region,
            org_id: dataset.org_id,
            project_id: dataset.project_id,
            created_at: dataset.created_at,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DatasetsCache {
    pub fetched_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub datasets: Vec<DatasetEntry>,
}

impl DatasetsCache {
    pub fn is_fresh(&self) -> bool {
        self.fetched_at
            .map(|t| Utc::now() - t < Duration::minutes(TTL_MINUTES))
            .unwrap_or(false)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.datasets.iter().any(|d| d.name == name)
    }

    pub fn get_regions(&self, name: &str) -> Vec<&str> {
        self.datasets
            .iter()
            .filter(|d| d.name == name)
            .map(|d| d.region.as_str())
            .collect()
    }

    pub fn get(&self, name: impl Into<String>) -> Option<&DatasetEntry> {
        let name = name.into();
        let mut matches = self.datasets.iter().filter(|d| d.name == name);
        let first = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some(first)
    }

    pub fn set_all(&mut self, datasets: impl IntoIterator<Item = DatasetEntry>) {
        self.datasets = datasets.into_iter().collect();
        self.fetched_at = Some(Utc::now());
    }

    pub fn insert(&mut self, dataset: DatasetEntry) {
        self.datasets
            .retain(|d| !(d.name == dataset.name && d.region == dataset.region));
        self.datasets.push(dataset);
    }

    pub fn remove(&mut self, name: &str) {
        self.datasets.retain(|d| d.name != name);
    }
}

pub fn cache_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("topk").join("datasets.toml"))
}

pub(crate) fn load() -> DatasetsCache {
    let path = match cache_path() {
        Some(p) => p,
        None => return DatasetsCache::default(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return DatasetsCache::default(),
    };
    toml::from_str(&content).unwrap_or_default()
}

pub(crate) fn save(cache: &DatasetsCache) -> Result<()> {
    let path =
        cache_path().ok_or_else(|| anyhow::anyhow!("could not determine config directory"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, toml::to_string_pretty(cache)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, region: &str) -> DatasetEntry {
        DatasetEntry {
            name: name.to_string(),
            region: region.to_string(),
            org_id: String::new(),
            project_id: String::new(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn fresh_cache(datasets: &[(&str, &str)]) -> DatasetsCache {
        let mut c = DatasetsCache::default();
        c.set_all(datasets.iter().map(|(n, r)| entry(n, r)));
        c
    }

    #[test]
    fn is_fresh_when_just_populated() {
        let cache = fresh_cache(&[("ds", "us-east-1")]);
        assert!(cache.is_fresh());
    }

    #[test]
    fn is_stale_when_fetched_at_is_old() {
        let mut cache = fresh_cache(&[("ds", "us-east-1")]);
        cache.fetched_at = Some(Utc::now() - Duration::minutes(TTL_MINUTES + 1));
        assert!(!cache.is_fresh());
    }

    #[test]
    fn is_stale_when_never_populated() {
        let cache = DatasetsCache::default();
        assert!(!cache.is_fresh());
    }

    #[test]
    fn get_regions_returns_value_for_known_dataset() {
        let cache = fresh_cache(&[("my-ds", "eu-west-1")]);
        assert_eq!(cache.get_regions("my-ds"), vec!["eu-west-1"]);
    }

    #[test]
    fn get_regions_returns_multiple_when_same_name_in_different_regions() {
        let cache = fresh_cache(&[("ds", "us-east-1"), ("ds", "eu-west-1")]);
        let mut regions = cache.get_regions("ds");
        regions.sort();
        assert_eq!(regions, vec!["eu-west-1", "us-east-1"]);
    }

    #[test]
    fn get_regions_returns_empty_for_unknown_dataset() {
        let cache = fresh_cache(&[("other", "us-east-1")]);
        assert!(cache.get_regions("missing").is_empty());
    }

    #[test]
    fn contains_returns_true_for_known_dataset() {
        let cache = fresh_cache(&[("ds", "us-east-1")]);
        assert!(cache.contains("ds"));
        assert!(!cache.contains("other"));
    }

    #[test]
    fn get_returns_single_cached_dataset() {
        let cache = fresh_cache(&[("ds", "us-east-1")]);
        assert_eq!(
            cache.get("ds").map(|d| d.region.as_str()),
            Some("us-east-1")
        );
    }

    #[test]
    fn get_returns_none_when_dataset_exists_in_multiple_regions() {
        let cache = fresh_cache(&[("ds", "us-east-1"), ("ds", "eu-west-1")]);
        assert!(cache.get("ds").is_none());
    }

    #[test]
    fn set_all_replaces_existing_entries() {
        let mut cache = fresh_cache(&[("old", "us-east-1")]);
        cache.set_all([entry("new", "eu-west-1")]);
        assert!(cache.get_regions("new").contains(&"eu-west-1"));
        assert!(cache.get_regions("old").is_empty());
    }

    #[test]
    fn insert_adds_entry_without_clearing_others() {
        let mut cache = fresh_cache(&[("a", "us-east-1")]);
        cache.insert(entry("b", "eu-west-1"));
        assert_eq!(cache.get_regions("a"), vec!["us-east-1"]);
        assert_eq!(cache.get_regions("b"), vec!["eu-west-1"]);
    }

    #[test]
    fn insert_replaces_same_name_and_region() {
        let mut cache = fresh_cache(&[("a", "us-east-1")]);
        let mut updated = entry("a", "us-east-1");
        updated.created_at = "updated".to_string();
        cache.insert(updated);
        assert_eq!(cache.get("a").unwrap().created_at, "updated");
    }

    #[test]
    fn remove_deletes_all_regions_for_name() {
        let mut cache = fresh_cache(&[("a", "us-east-1"), ("a", "eu-west-1"), ("b", "us-east-1")]);
        cache.remove("a");
        assert!(cache.get_regions("a").is_empty());
        assert_eq!(cache.get_regions("b"), vec!["us-east-1"]);
    }

    #[test]
    fn serialization_roundtrip() {
        let cache = fresh_cache(&[("ds1", "us-east-1"), ("ds2", "eu-west-1")]);
        let toml = toml::to_string_pretty(&cache).unwrap();
        let restored: DatasetsCache = toml::from_str(&toml).unwrap();
        assert!(restored.is_fresh());
        assert_eq!(restored.get_regions("ds1"), vec!["us-east-1"]);
        assert_eq!(restored.get_regions("ds2"), vec!["eu-west-1"]);
    }
}
