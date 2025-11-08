use std::time::Duration;

use async_trait::async_trait;

use crate::data::Document;
use crate::providers::{Provider, ProviderLike};
use crate::providers::{PythonProvider, Query};

pub const PY_CODE: &str = include_str!("topk.py");

#[derive(Clone)]
pub struct TopkPyProvider {
    /// Python interpreter
    py: PythonProvider,
}

impl TopkPyProvider {
    /// Creates a new TopkPyProvider.
    pub async fn new() -> anyhow::Result<Provider> {
        let py = PythonProvider::new(PY_CODE).await?;

        Ok(Provider::TopkPy(TopkPyProvider { py }))
    }
}

#[async_trait]
impl ProviderLike for TopkPyProvider {
    async fn setup(&self, collection: String) -> anyhow::Result<()> {
        self.py.setup(collection).await
    }

    async fn ping(&self, collection: String) -> anyhow::Result<Duration> {
        self.py.ping(collection).await
    }

    async fn query(&self, collection: String, query: Query) -> anyhow::Result<Vec<Document>> {
        self.py.query(collection, query).await
    }

    async fn delete_by_id(&self, collection: String, ids: Vec<String>) -> anyhow::Result<()> {
        self.py.delete_by_id(collection, ids).await
    }

    async fn query_by_id(
        &self,
        collection: String,
        id: String,
    ) -> anyhow::Result<Option<Document>> {
        self.py.query_by_id(collection, id).await
    }

    async fn upsert(&self, collection: String, batch: Vec<Document>) -> anyhow::Result<()> {
        self.py.upsert(collection, batch).await
    }

    async fn list_collections(&self) -> anyhow::Result<Vec<String>> {
        self.py.list_collections().await
    }

    async fn delete_collection(&self, name: String) -> anyhow::Result<()> {
        self.py.delete_collection(name).await
    }

    async fn close(&self) -> anyhow::Result<()> {
        self.py.close().await
    }
}
