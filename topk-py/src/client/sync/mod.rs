mod ask;
mod collection;
mod collections;
mod dataset;
mod datasets;
mod runtime;

use std::sync::Arc;

pub use ask::{ask, ask_stream, AskIterator};
pub use collection::CollectionClient;
pub use collections::CollectionsClient;
pub use dataset::DatasetClient;
pub use datasets::DatasetsClient;

use crate::{
    client::{sync::runtime::Runtime, topk_client, NativeRetryConfig},
    data::ask::{AskResponseMessage, Effort, Sources},
    expr::logical::LogicalExpr,
};

use pyo3::{prelude::Python, pyclass, pymethods, PyResult};

#[pyclass]
pub struct Client {
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (api_key, region, host="topk.io".into(), https=true, retry_config=None))]
    pub fn new(
        api_key: String,
        region: String,
        host: String,
        https: bool,
        retry_config: Option<NativeRetryConfig>,
    ) -> Self {
        let runtime = Arc::new(Runtime::new().expect("failed to create runtime"));

        let client = topk_client(api_key, region, host, https, retry_config.map(|c| c.config));

        Self { runtime, client }
    }

    pub fn collection(&self, collection: String) -> PyResult<CollectionClient> {
        Ok(CollectionClient::new(
            self.runtime.clone(),
            self.client.clone(),
            collection,
        ))
    }

    pub fn collections(&self) -> PyResult<CollectionsClient> {
        Ok(CollectionsClient::new(
            self.runtime.clone(),
            self.client.clone(),
        ))
    }

    pub fn dataset(&self, dataset: String) -> PyResult<DatasetClient> {
        Ok(DatasetClient::new(
            self.runtime.clone(),
            self.client.clone(),
            dataset,
        ))
    }

    pub fn datasets(&self) -> PyResult<DatasetsClient> {
        Ok(DatasetsClient::new(
            self.runtime.clone(),
            self.client.clone(),
        ))
    }

    #[pyo3(signature = (query, sources, filter=None, effort=None))]
    pub fn ask(
        &self,
        py: Python<'_>,
        query: String,
        sources: Sources,
        filter: Option<LogicalExpr>,
        effort: Option<Effort>,
    ) -> PyResult<AskResponseMessage> {
        ask(
            self.runtime.clone(),
            self.client.clone(),
            py,
            query,
            sources.into(),
            filter,
            effort.unwrap_or(Effort::Medium),
        )
    }

    #[pyo3(signature = (query, sources, filter=None, effort=None))]
    pub fn ask_stream(
        &self,
        query: String,
        sources: Sources,
        filter: Option<LogicalExpr>,
        effort: Option<Effort>,
    ) -> PyResult<AskIterator> {
        ask_stream(
            self.runtime.clone(),
            self.client.clone(),
            query,
            sources.into(),
            filter,
            effort.unwrap_or(Effort::Medium),
        )
    }
}
