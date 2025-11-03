use std::time::Duration;

use async_trait::async_trait;

use ::topk_rs::proto::v1::data::Document;

pub mod topk_py;
pub mod topk_rs;
pub mod tpuf_py;

#[async_trait]
pub trait ProviderLike: Send + Sync + 'static {
    /// Setup the state in the provider.
    async fn setup(&self) -> Result<(), anyhow::Error>;

    /// Ping the provider, returns duration in milliseconds.
    /// This can be used to estimate if we are running the right region.
    async fn ping(&self) -> Result<Duration, anyhow::Error>;

    /// Upsert documents into the provider.
    async fn upsert(&self, batch: Vec<Document>) -> Result<(), anyhow::Error>;

    /// Close the provider.
    async fn close(&self) -> Result<(), anyhow::Error>;
}

#[derive(Clone)]
pub enum Provider {
    /// TopK
    TopkRs(topk_rs::TopkRsProvider),
    TopkPy(topk_py::TopkPyProvider),
    //
    TpufPy(tpuf_py::TpufPyProvider),
}

#[async_trait]
impl ProviderLike for Provider {
    async fn setup(&self) -> Result<(), anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.setup().await,
            Provider::TopkPy(p) => p.setup().await,
            Provider::TpufPy(p) => p.setup().await,
        }
    }

    async fn ping(&self) -> Result<Duration, anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.ping().await,
            Provider::TopkPy(p) => p.ping().await,
            Provider::TpufPy(p) => p.ping().await,
        }
    }

    async fn upsert(&self, batch: Vec<Document>) -> Result<(), anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.upsert(batch.clone()).await,
            Provider::TopkPy(p) => p.upsert(batch.clone()).await,
            Provider::TpufPy(p) => p.upsert(batch.clone()).await,
        }
    }

    async fn close(&self) -> Result<(), anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.close().await,
            Provider::TopkPy(p) => p.close().await,
            Provider::TpufPy(p) => p.close().await,
        }
    }
}

#[macro_export]
macro_rules! run_python {
    ($f:expr) => {
        tokio::task::spawn_blocking(move || Python::with_gil(|py: Python<'_>| $f(py))).await?
    };
}
