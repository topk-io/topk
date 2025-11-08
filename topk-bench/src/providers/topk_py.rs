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
    pub async fn new(collection: String) -> anyhow::Result<Provider> {
        let py = PythonProvider::new(PY_CODE, collection).await?;

        Ok(Provider::TopkPy(TopkPyProvider { py }))
    }
}

#[async_trait]
impl ProviderLike for TopkPyProvider {
    async fn setup(&self) -> anyhow::Result<()> {
        self.py.setup().await
    }

    async fn ping(&self) -> anyhow::Result<Duration> {
        self.py.ping().await
    }

    async fn query(&self, query: Query) -> anyhow::Result<Vec<Document>> {
        self.py.query(query).await
    }

    async fn delete_by_id(&self, ids: Vec<String>) -> anyhow::Result<()> {
        self.py.delete_by_id(ids).await
    }

    async fn query_by_id(&self, id: String) -> anyhow::Result<Option<Document>> {
        self.py.query_by_id(id).await
    }

    async fn upsert(&self, batch: Vec<Document>) -> anyhow::Result<()> {
        self.py.upsert(batch).await
    }

    async fn close(&self) -> anyhow::Result<()> {
        self.py.close().await
    }
}
