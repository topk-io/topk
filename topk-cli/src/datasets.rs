use async_trait::async_trait;
use topk_rs::{
    proto::v1::control::{CreateDatasetResponse, GetDatasetResponse, ListDatasetsResponse},
    Client, Error,
};

use crate::cache::{cache_path, Cache, DatasetEntry};

#[async_trait(?Send)]
pub trait DatasetsClient {
    async fn list(&mut self) -> Result<ListDatasetsResponse, Error>;
    async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error>;
    async fn create(&mut self, name: &str, region: &str) -> Result<CreateDatasetResponse, Error>;
    async fn delete(&mut self, name: &str) -> Result<(), Error>;
}

struct RealDatasetsClient {
    client: Client,
}

impl RealDatasetsClient {
    fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait(?Send)]
impl DatasetsClient for RealDatasetsClient {
    async fn list(&mut self) -> Result<ListDatasetsResponse, Error> {
        Ok(self.client.datasets().list().await?.into_inner())
    }

    async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error> {
        Ok(self.client.datasets().get(name).await?.into_inner())
    }

    async fn create(&mut self, name: &str, region: &str) -> Result<CreateDatasetResponse, Error> {
        Ok(self
            .client
            .datasets()
            .create(name, Some(region.to_string()))
            .await?
            .into_inner())
    }

    async fn delete(&mut self, name: &str) -> Result<(), Error> {
        self.client.datasets().delete(name).await?;
        Ok(())
    }
}

struct CachedDatasetsClient<B> {
    client: B,
    cache: Cache,
}

impl<B> CachedDatasetsClient<B> {
    fn new(client: B, cache: Cache) -> Self {
        Self { client, cache }
    }

    fn persist(&self) {
        if let Err(err) = self.cache.save() {
            eprintln!("warning: failed to persist dataset index: {err}");
        }
    }
}

#[async_trait(?Send)]
impl<B> DatasetsClient for CachedDatasetsClient<B>
where
    B: DatasetsClient,
{
    async fn list(&mut self) -> Result<ListDatasetsResponse, Error> {
        let response = self.client.list().await?;
        self.cache
            .set_all(response.datasets.iter().cloned().map(DatasetEntry::from));
        self.persist();
        Ok(response)
    }

    async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error> {
        if let Some(entry) = self.cache.lookup(name) {
            return Ok(GetDatasetResponse {
                dataset: Some(entry.clone().into()),
            });
        }

        let response = self.client.get(name).await?;
        if let Some(dataset) = response.dataset.as_ref().cloned() {
            self.cache.insert(dataset.into());
            self.persist();
        }
        Ok(response)
    }

    async fn create(&mut self, name: &str, region: &str) -> Result<CreateDatasetResponse, Error> {
        let response = self.client.create(name, region).await?;
        if let Some(dataset) = response.dataset.as_ref().cloned() {
            self.cache.insert(dataset.into());
            self.persist();
        }
        Ok(response)
    }

    async fn delete(&mut self, name: &str) -> Result<(), Error> {
        self.client.delete(name).await?;
        self.cache.remove(name);
        self.persist();
        Ok(())
    }
}

pub fn make_cached_datasets_client(client: Client) -> impl DatasetsClient {
    CachedDatasetsClient::new(RealDatasetsClient::new(client), Cache::new(cache_path()))
}

pub async fn get_region<C: DatasetsClient + ?Sized>(
    client: &mut C,
    name: &str,
) -> Result<String, Error> {
    let response = client.get(name).await?;
    response
        .dataset
        .as_ref()
        .map(|dataset| dataset.region.clone())
        .ok_or_else(|| Error::MalformedResponse("dataset missing from get response".to_string()))
}

/// Resolves a single region shared by every dataset in `datasets`.
///
/// Errors if `datasets` is empty, if any dataset cannot be resolved, or if the
/// datasets span more than one region.
pub async fn ensure_unique_region<C: DatasetsClient + ?Sized>(
    client: &mut C,
    datasets: Vec<String>,
) -> Result<String, Error> {
    let mut pairs: Vec<(String, String)> = Vec::with_capacity(datasets.len());
    for name in datasets {
        let region = get_region(client, &name).await?;
        pairs.push((name.clone(), region));
    }

    let mut iter = pairs.iter();
    let (_, first) = iter
        .next()
        .ok_or_else(|| Error::Input(anyhow::anyhow!("at least one dataset is required")))?;

    if iter.any(|(_, r)| r != first) {
        let details = pairs
            .iter()
            .map(|(name, region)| format!("{name} ({region})"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(Error::Input(anyhow::anyhow!(
            "cannot query datasets across regions: {details}"
        )));
    }

    Ok(first.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use topk_rs::proto::v1::control::Dataset;

    #[derive(Default)]
    struct FakeDatasetsClient {
        datasets: BTreeMap<String, Dataset>,
        list_calls: usize,
        get_calls: usize,
        create_calls: usize,
        delete_calls: usize,
    }

    impl FakeDatasetsClient {
        fn with_dataset(dataset: Dataset) -> Self {
            let mut remote = Self::default();
            remote.datasets.insert(dataset.name.clone(), dataset);
            remote
        }
    }

    #[async_trait(?Send)]
    impl DatasetsClient for FakeDatasetsClient {
        async fn list(&mut self) -> Result<ListDatasetsResponse, Error> {
            self.list_calls += 1;
            Ok(ListDatasetsResponse {
                datasets: self.datasets.values().cloned().collect(),
            })
        }

        async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error> {
            self.get_calls += 1;
            let dataset = self
                .datasets
                .get(name)
                .cloned()
                .ok_or(Error::DatasetNotFound)?;
            Ok(GetDatasetResponse {
                dataset: Some(dataset),
            })
        }

        async fn create(
            &mut self,
            name: &str,
            region: &str,
        ) -> Result<CreateDatasetResponse, Error> {
            self.create_calls += 1;
            let dataset = dataset(name, region);
            self.datasets.insert(name.to_string(), dataset.clone());
            Ok(CreateDatasetResponse {
                dataset: Some(dataset),
            })
        }

        async fn delete(&mut self, name: &str) -> Result<(), Error> {
            self.delete_calls += 1;
            self.datasets.remove(name).ok_or(Error::DatasetNotFound)?;
            Ok(())
        }
    }

    fn dataset(name: &str, region: &str) -> Dataset {
        Dataset {
            name: name.to_string(),
            region: region.to_string(),
            org_id: "org".to_string(),
            project_id: "project".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn indexed(dataset: Dataset) -> Cache {
        let mut cache = Cache::default();
        cache.insert(dataset.into());
        cache
    }

    #[tokio::test]
    async fn list_repopulates_index_from_remote() {
        let backend = FakeDatasetsClient::with_dataset(dataset("ds", "us-east-1"));
        let mut client = CachedDatasetsClient::new(backend, Cache::default());

        let response = client.list().await.unwrap();

        assert_eq!(response.datasets.len(), 1);
        assert!(client.cache.lookup("ds").is_some());
    }

    #[tokio::test]
    async fn get_uses_index_before_remote() {
        let backend = FakeDatasetsClient::with_dataset(dataset("remote", "eu-west-1"));
        let mut client =
            CachedDatasetsClient::new(backend, indexed(dataset("cached", "us-east-1")));

        let response = client.get("cached").await.unwrap();

        assert_eq!(response.dataset.unwrap().region, "us-east-1");
        assert_eq!(client.client.get_calls, 0);
    }

    #[tokio::test]
    async fn get_fetches_remote_and_updates_index_on_miss() {
        let backend = FakeDatasetsClient::with_dataset(dataset("ds", "us-east-1"));
        let mut client = CachedDatasetsClient::new(backend, Cache::default());

        let response = client.get("ds").await.unwrap();

        assert_eq!(response.dataset.unwrap().region, "us-east-1");
        assert_eq!(client.client.get_calls, 1);
        assert_eq!(
            client.cache.lookup("ds").map(|d| d.region.as_str()),
            Some("us-east-1")
        );
    }

    #[tokio::test]
    async fn create_adds_remote_dataset_to_index() {
        let mut client = CachedDatasetsClient::new(FakeDatasetsClient::default(), Cache::default());

        let response = client.create("ds", "us-east-1").await.unwrap();

        assert_eq!(response.dataset.unwrap().name, "ds");
        assert_eq!(client.client.create_calls, 1);
        assert!(client.cache.lookup("ds").is_some());
    }

    #[tokio::test]
    async fn delete_removes_dataset_from_index_after_remote_success() {
        let mut client = CachedDatasetsClient::new(
            FakeDatasetsClient::with_dataset(dataset("ds", "us-east-1")),
            indexed(dataset("ds", "us-east-1")),
        );

        client.delete("ds").await.unwrap();

        assert_eq!(client.client.delete_calls, 1);
        assert!(client.cache.lookup("ds").is_none());
    }

    #[tokio::test]
    async fn get_region_reads_dataset_region() {
        let backend = FakeDatasetsClient::with_dataset(dataset("ds", "us-east-1"));
        let mut client = CachedDatasetsClient::new(backend, Cache::default());

        assert_eq!(get_region(&mut client, "ds").await.unwrap(), "us-east-1");
    }

    #[tokio::test]
    async fn ensure_unique_region_returns_shared_region() {
        let mut backend = FakeDatasetsClient::default();
        backend
            .datasets
            .insert("a".into(), dataset("a", "us-east-1"));
        backend
            .datasets
            .insert("b".into(), dataset("b", "us-east-1"));
        let mut client = CachedDatasetsClient::new(backend, Cache::default());

        let region = ensure_unique_region(&mut client, vec!["a".into(), "b".into()])
            .await
            .unwrap();

        assert_eq!(region, "us-east-1");
    }

    #[tokio::test]
    async fn ensure_unique_region_errors_when_regions_differ() {
        let mut backend = FakeDatasetsClient::default();
        backend
            .datasets
            .insert("a".into(), dataset("a", "us-east-1"));
        backend
            .datasets
            .insert("b".into(), dataset("b", "eu-west-1"));
        let mut client = CachedDatasetsClient::new(backend, Cache::default());

        let err = ensure_unique_region(&mut client, vec!["a".into(), "b".into()])
            .await
            .unwrap_err();

        let msg = err.to_string();
        assert!(
            msg.contains("cannot query datasets across regions"),
            "{msg}"
        );
        assert!(msg.contains("a (us-east-1)"), "{msg}");
        assert!(msg.contains("b (eu-west-1)"), "{msg}");
    }

    #[tokio::test]
    async fn ensure_unique_region_errors_on_empty_input() {
        let mut client = CachedDatasetsClient::new(FakeDatasetsClient::default(), Cache::default());

        let err = ensure_unique_region(&mut client, vec![]).await.unwrap_err();

        assert!(err.to_string().contains("at least one dataset is required"));
    }
}
