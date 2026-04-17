use async_trait::async_trait;
use topk_rs::{
    proto::v1::control::{
        CreateDatasetResponse, DeleteDatasetResponse, GetDatasetResponse, ListDatasetsResponse,
    },
    Client, Error,
};

use crate::cache::{self, DatasetEntry, DatasetsCache};

#[async_trait(?Send)]
pub trait DatasetsClient {
    async fn list(&mut self) -> Result<ListDatasetsResponse, Error>;
    async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error>;
    async fn create(&mut self, name: &str, region: &str) -> Result<CreateDatasetResponse, Error>;
    async fn delete(&mut self, name: &str) -> Result<(), Error>;

    async fn get_region(&mut self, name: &str) -> Result<String, Error> {
        let response = self.get(name).await?;
        response
            .dataset
            .as_ref()
            .map(|dataset| dataset.region.clone())
            .ok_or_else(|| Error::MalformedResponse("dataset missing from get response".to_string()))
    }
}

#[async_trait(?Send)]
trait RemoteDatasetsClient {
    async fn list(&mut self) -> Result<ListDatasetsResponse, Error>;
    async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error>;
    async fn create(&mut self, name: &str, region: &str) -> Result<CreateDatasetResponse, Error>;
    async fn delete(&mut self, name: &str) -> Result<DeleteDatasetResponse, Error>;
}

struct TopkRemoteDatasetsClient {
    global_client: Client,
}

impl TopkRemoteDatasetsClient {
    fn new(global_client: Client) -> Self {
        Self { global_client }
    }
}

#[async_trait(?Send)]
impl RemoteDatasetsClient for TopkRemoteDatasetsClient {
    async fn list(&mut self) -> Result<ListDatasetsResponse, Error> {
        Ok(self.global_client.datasets().list().await?.into_inner())
    }

    async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error> {
        Ok(self.global_client.datasets().get(name).await?.into_inner())
    }

    async fn create(&mut self, name: &str, region: &str) -> Result<CreateDatasetResponse, Error> {
        Ok(self
            .global_client
            .datasets()
            .create(name, Some(region.to_string()))
            .await?
            .into_inner())
    }

    async fn delete(&mut self, name: &str) -> Result<DeleteDatasetResponse, Error> {
        Ok(self.global_client.datasets().delete(name).await?.into_inner())
    }
}

struct CachedDatasetsClient<R> {
    remote: R,
    cache: DatasetsCache,
}

impl<R> CachedDatasetsClient<R> {
    fn with_remote(remote: R, cache: DatasetsCache) -> Self {
        Self { remote, cache }
    }

    fn persist_cache(&self) {
        let _ = cache::save(&self.cache);
    }
}

#[async_trait(?Send)]
impl<R> DatasetsClient for CachedDatasetsClient<R>
where
    R: RemoteDatasetsClient,
{
    async fn list(&mut self) -> Result<ListDatasetsResponse, Error> {
        let response = self.remote.list().await?;
        self.cache
            .set_all(response.datasets.iter().cloned().map(DatasetEntry::from));
        self.persist_cache();
        Ok(response)
    }

    async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error> {
        if let Some(dataset) = self.cache.get(name).cloned() {
            return Ok(GetDatasetResponse {
                dataset: Some(dataset.into()),
            });
        }

        let response = self.remote.get(name).await?;
        if let Some(dataset) = response.dataset.as_ref().cloned() {
            self.cache.insert(dataset.into());
            self.persist_cache();
        }
        Ok(response)
    }

    async fn create(&mut self, name: &str, region: &str) -> Result<CreateDatasetResponse, Error> {
        let response = self.remote.create(name, region).await?;
        if let Some(dataset) = response.dataset.as_ref().cloned() {
            self.cache.insert(dataset.into());
            self.persist_cache();
        }
        Ok(response)
    }

    async fn delete(&mut self, name: &str) -> Result<(), Error> {
        self.remote.delete(name).await?;
        self.cache.remove(name);
        self.persist_cache();
        Ok(())
    }
}

pub struct TopkDatasetsClient {
    inner: CachedDatasetsClient<TopkRemoteDatasetsClient>,
}

impl TopkDatasetsClient {
    pub fn new(global_client: Client) -> Self {
        Self {
            inner: CachedDatasetsClient::with_remote(
                TopkRemoteDatasetsClient::new(global_client),
                cache::load(),
            ),
        }
    }
}

#[async_trait(?Send)]
impl DatasetsClient for TopkDatasetsClient {
    async fn list(&mut self) -> Result<ListDatasetsResponse, Error> {
        self.inner.list().await
    }

    async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error> {
        self.inner.get(name).await
    }

    async fn create(&mut self, name: &str, region: &str) -> Result<CreateDatasetResponse, Error> {
        self.inner.create(name, region).await
    }

    async fn delete(&mut self, name: &str) -> Result<(), Error> {
        self.inner.delete(name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use topk_rs::proto::v1::control::Dataset;

    #[derive(Default)]
    struct FakeRemoteDatasetsClient {
        datasets: BTreeMap<String, Dataset>,
        list_calls: usize,
        get_calls: usize,
        create_calls: usize,
        delete_calls: usize,
    }

    impl FakeRemoteDatasetsClient {
        fn with_dataset(dataset: Dataset) -> Self {
            let mut remote = Self::default();
            remote.datasets.insert(dataset.name.clone(), dataset);
            remote
        }
    }

    #[async_trait(?Send)]
    impl RemoteDatasetsClient for FakeRemoteDatasetsClient {
        async fn list(&mut self) -> Result<ListDatasetsResponse, Error> {
            self.list_calls += 1;
            Ok(ListDatasetsResponse {
                datasets: self.datasets.values().cloned().collect(),
            })
        }

        async fn get(&mut self, name: &str) -> Result<GetDatasetResponse, Error> {
            self.get_calls += 1;
            let dataset = self.datasets.get(name).cloned().ok_or(Error::DatasetNotFound)?;
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

        async fn delete(&mut self, name: &str) -> Result<DeleteDatasetResponse, Error> {
            self.delete_calls += 1;
            self.datasets.remove(name).ok_or(Error::DatasetNotFound)?;
            Ok(DeleteDatasetResponse {})
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

    fn cached(dataset: Dataset) -> DatasetsCache {
        let mut cache = DatasetsCache::default();
        cache.insert(dataset.into());
        cache
    }

    #[tokio::test]
    async fn list_repopulates_cache_from_remote() {
        let remote = FakeRemoteDatasetsClient::with_dataset(dataset("ds", "us-east-1"));
        let mut client = CachedDatasetsClient::with_remote(remote, DatasetsCache::default());

        let response = client.list().await.unwrap();

        assert_eq!(response.datasets.len(), 1);
        assert!(client.cache.contains("ds"));
    }

    #[tokio::test]
    async fn get_uses_cache_before_remote() {
        let remote = FakeRemoteDatasetsClient::with_dataset(dataset("remote", "eu-west-1"));
        let mut client =
            CachedDatasetsClient::with_remote(remote, cached(dataset("cached", "us-east-1")));

        let response = client.get("cached").await.unwrap();

        assert_eq!(response.dataset.unwrap().region, "us-east-1");
        assert_eq!(client.remote.get_calls, 0);
    }

    #[tokio::test]
    async fn get_fetches_remote_and_updates_cache_on_miss() {
        let remote = FakeRemoteDatasetsClient::with_dataset(dataset("ds", "us-east-1"));
        let mut client = CachedDatasetsClient::with_remote(remote, DatasetsCache::default());

        let response = client.get("ds").await.unwrap();

        assert_eq!(response.dataset.unwrap().region, "us-east-1");
        assert_eq!(client.remote.get_calls, 1);
        assert_eq!(client.cache.get("ds").unwrap().region, "us-east-1");
    }

    #[tokio::test]
    async fn create_adds_remote_dataset_to_cache() {
        let mut client = CachedDatasetsClient::with_remote(
            FakeRemoteDatasetsClient::default(),
            DatasetsCache::default(),
        );

        let response = client.create("ds", "us-east-1").await.unwrap();

        assert_eq!(response.dataset.unwrap().name, "ds");
        assert_eq!(client.remote.create_calls, 1);
        assert_eq!(client.cache.get("ds").unwrap().region, "us-east-1");
    }

    #[tokio::test]
    async fn delete_removes_dataset_from_cache_after_remote_success() {
        let mut client = CachedDatasetsClient::with_remote(
            FakeRemoteDatasetsClient::with_dataset(dataset("ds", "us-east-1")),
            cached(dataset("ds", "us-east-1")),
        );

        client.delete("ds").await.unwrap();

        assert_eq!(client.remote.delete_calls, 1);
        assert!(client.cache.get("ds").is_none());
    }

    #[tokio::test]
    async fn get_region_uses_trait_default_implementation() {
        let remote = FakeRemoteDatasetsClient::with_dataset(dataset("ds", "us-east-1"));
        let mut client = CachedDatasetsClient::with_remote(remote, DatasetsCache::default());

        assert_eq!(client.get_region("ds").await.unwrap(), "us-east-1");
    }
}
