use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use topk_rs::Error;

use crate::config::{load_toml_or_default, save_toml};

const DATASET_REGION_CACHE_TTL: Duration = Duration::from_mins(5);

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CachedDatasetRegion {
    name: String,
    region: String,
    cached_at: u64,
}

impl CachedDatasetRegion {
    fn new(name: String, region: String, cached_at: u64) -> Self {
        Self {
            name,
            region,
            cached_at,
        }
    }

    fn is_fresh(&self, now_epoch_seconds: u64) -> bool {
        now_epoch_seconds.saturating_sub(self.cached_at) <= DATASET_REGION_CACHE_TTL.as_secs()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DatasetRegionStore {
    #[serde(default)]
    datasets: Vec<CachedDatasetRegion>,
}

impl DatasetRegionStore {
    fn lookup(&self, name: &str, now_epoch_seconds: u64) -> Option<&str> {
        self.datasets
            .iter()
            .find(|dataset| dataset.name == name && dataset.is_fresh(now_epoch_seconds))
            .map(|dataset| dataset.region.as_str())
    }

    fn set_all(
        &mut self,
        datasets: impl IntoIterator<Item = (String, String)>,
        now_epoch_seconds: u64,
    ) {
        self.datasets = datasets
            .into_iter()
            .map(|(name, region)| CachedDatasetRegion::new(name, region, now_epoch_seconds))
            .collect();
    }

    fn insert(&mut self, name: String, region: String, now_epoch_seconds: u64) {
        self.datasets.retain(|dataset| dataset.name != name);
        self.datasets
            .push(CachedDatasetRegion::new(name, region, now_epoch_seconds));
    }

    fn remove(&mut self, name: &str) {
        self.datasets.retain(|dataset| dataset.name != name);
    }

    fn prune_expired(&mut self, now_epoch_seconds: u64) {
        self.datasets
            .retain(|dataset| dataset.is_fresh(now_epoch_seconds));
    }
}

#[derive(Default)]
pub struct DatasetRegionCache {
    path: Option<PathBuf>,
    store: DatasetRegionStore,
}

impl DatasetRegionCache {
    pub fn new(path: Option<PathBuf>) -> Self {
        let store = load_toml_or_default(path.clone(), |p, err| {
            eprintln!(
                "warning: failed to parse file {}: {err}, defaulting to empty cache",
                p.display()
            );
        });

        Self { path, store }
    }

    fn current_epoch_seconds(&self) -> u64 {
        now_epoch_seconds()
    }

    pub fn save(&mut self) -> Result<(), Error> {
        self.store.prune_expired(self.current_epoch_seconds());
        save_toml(self.path.clone(), &self.store)
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.store.lookup(name, self.current_epoch_seconds())
    }

    pub fn set_all(&mut self, datasets: impl IntoIterator<Item = (String, String)>) {
        self.store.set_all(datasets, self.current_epoch_seconds());
    }

    pub fn insert(&mut self, name: impl Into<String>, region: impl Into<String>) {
        self.store
            .insert(name.into(), region.into(), self.current_epoch_seconds());
    }

    pub fn remove(&mut self, name: &str) {
        self.store.remove(name);
    }
}

pub fn dataset_region_cache_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("topk").join("datasets.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000;

    fn later() -> u64 {
        NOW + DATASET_REGION_CACHE_TTL.as_secs() + 1
    }

    fn create_store(datasets: &[(&str, &str)], cached_at_epoch_seconds: u64) -> DatasetRegionStore {
        let mut i = DatasetRegionStore::default();
        i.datasets = datasets
            .iter()
            .map(|(name, region)| {
                CachedDatasetRegion::new(
                    (*name).to_string(),
                    (*region).to_string(),
                    cached_at_epoch_seconds,
                )
            })
            .collect();
        i
    }

    #[test]
    fn lookup_returns_region_for_known_name() {
        let store = create_store(&[("ds", "aws-us-east-1-elastica")], NOW);
        assert_eq!(store.lookup("ds", NOW), Some("aws-us-east-1-elastica"));
    }

    #[test]
    fn lookup_returns_none_for_unknown_name() {
        let store = create_store(&[("other", "aws-us-east-1-elastica")], NOW);
        assert_eq!(store.lookup("missing", NOW), None);
    }

    #[test]
    fn lookup_returns_none_for_expired_entry() {
        let store = create_store(&[("ds", "aws-us-east-1-elastica")], NOW);
        assert_eq!(store.lookup("ds", later()), None);
    }

    #[test]
    fn set_all_replaces_existing_entries() {
        let mut store = create_store(&[("old", "aws-us-east-1-elastica")], NOW);
        store.set_all(
            [("new".to_string(), "aws-eu-central-1-monstera".to_string())],
            NOW,
        );
        assert_eq!(store.lookup("new", NOW), Some("aws-eu-central-1-monstera"));
        assert_eq!(store.lookup("old", NOW), None);
    }

    #[test]
    fn insert_adds_entry_without_clearing_others() {
        let mut store = create_store(&[("a", "aws-us-east-1-elastica")], NOW);
        store.insert(
            "b".to_string(),
            "aws-eu-central-1-monstera".to_string(),
            NOW,
        );
        assert_eq!(store.lookup("a", NOW), Some("aws-us-east-1-elastica"));
        assert_eq!(store.lookup("b", NOW), Some("aws-eu-central-1-monstera"));
    }

    #[test]
    fn insert_replaces_same_name() {
        let mut store = create_store(&[("a", "aws-us-east-1-elastica")], NOW);
        store.insert(
            "a".to_string(),
            "aws-eu-central-1-monstera".to_string(),
            NOW,
        );
        assert_eq!(store.lookup("a", NOW), Some("aws-eu-central-1-monstera"));
    }

    #[test]
    fn remove_deletes_entry_for_name() {
        let mut store = create_store(
            &[
                ("a", "aws-us-east-1-elastica"),
                ("b", "aws-eu-central-1-monstera"),
            ],
            NOW,
        );
        store.remove("a");
        assert_eq!(store.lookup("a", NOW), None);
        assert_eq!(store.lookup("b", NOW), Some("aws-eu-central-1-monstera"));
    }

    #[test]
    fn cache_uses_ttl_when_reading_entries() {
        let mut cache = DatasetRegionCache::default();
        cache.insert("ds", "aws-us-east-1-elastica");
        assert_eq!(cache.get("ds"), Some("aws-us-east-1-elastica"));

        cache.store.datasets[0].cached_at =
            now_epoch_seconds() - DATASET_REGION_CACHE_TTL.as_secs() - 1;
        assert_eq!(cache.get("ds"), None);
    }

    #[test]
    fn serialization_roundtrip() {
        let store = create_store(
            &[
                ("dataset-1", "aws-us-east-1-elastica"),
                ("dataset-2", "aws-eu-central-1-monstera"),
            ],
            NOW,
        );
        let toml = toml::to_string_pretty(&store).unwrap();
        let restored: DatasetRegionStore = toml::from_str(&toml).unwrap();
        assert_eq!(
            restored.lookup("dataset-1", NOW),
            Some("aws-us-east-1-elastica")
        );
        assert_eq!(
            restored.lookup("dataset-2", NOW),
            Some("aws-eu-central-1-monstera")
        );
    }

    #[test]
    fn save_prunes_expired_entries_from_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("datasets.toml");

        let mut cache = DatasetRegionCache::new(Some(path.clone()));
        cache.insert("fresh", "aws-us-east-1-elastica");
        cache.insert("stale", "aws-eu-central-1-monstera");
        cache.store.datasets[1].cached_at =
            now_epoch_seconds() - DATASET_REGION_CACHE_TTL.as_secs() - 1;
        cache.save().unwrap();

        let saved = std::fs::read_to_string(path).unwrap();
        assert!(saved.contains("fresh"));
        assert!(!saved.contains("stale"));
    }
}
