use std::sync::Arc;

use pyo3::{prelude::*, PyResult};

use crate::{
    client::{
        r#async::ask::{ask, ask_stream},
        r#async::search::{search, search_stream},
        topk_client, NativeRetryConfig,
    },
    data::ask::{Mode, Sources},
    expr::logical::LogicalExpr,
};

mod ask;
mod collection;
mod collections;
mod dataset;
mod datasets;
mod search;

pub use ask::AsyncAskIterator;
pub use search::AsyncSearchIterator;
pub use collection::AsyncCollectionClient;
pub use collections::AsyncCollectionsClient;
pub use dataset::AsyncDatasetClient;
pub use datasets::AsyncDatasetsClient;

#[pyclass]
pub struct AsyncClient {
    client: Arc<topk_rs::Client>,
}

#[pymethods]
impl AsyncClient {
    #[new]
    #[pyo3(signature = (api_key, region, host="topk.io".into(), https=true, retry_config=None))]
    pub fn new(
        api_key: String,
        region: String,
        host: String,
        https: bool,
        retry_config: Option<NativeRetryConfig>,
    ) -> Self {
        let client = topk_client(api_key, region, host, https, retry_config.map(|c| c.config));

        Self { client }
    }

    pub fn collection(&self, collection: String) -> PyResult<AsyncCollectionClient> {
        Ok(AsyncCollectionClient::new(
            self.client.clone(),
            Arc::new(collection),
        ))
    }

    pub fn collections(&self) -> PyResult<AsyncCollectionsClient> {
        Ok(AsyncCollectionsClient::new(self.client.clone()))
    }

    pub fn dataset(&self, dataset: String) -> PyResult<AsyncDatasetClient> {
        Ok(AsyncDatasetClient::new(self.client.clone(), dataset))
    }

    pub fn datasets(&self) -> PyResult<AsyncDatasetsClient> {
        Ok(AsyncDatasetsClient::new(self.client.clone()))
    }

    #[pyo3(signature = (query, sources, filter=None, mode=None, select_fields=None))]
    pub fn ask(
        &self,
        py: Python<'_>,
        query: String,
        sources: Sources,
        filter: Option<LogicalExpr>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        ask(
            self.client.clone(),
            py,
            query,
            sources.into(),
            filter,
            mode,
            select_fields,
        )
    }

    #[pyo3(signature = (query, sources, filter=None, mode=None, select_fields=None))]
    pub fn ask_stream(
        &self,
        py: Python<'_>,
        query: String,
        sources: Sources,
        filter: Option<LogicalExpr>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        ask_stream(
            self.client.clone(),
            py,
            query,
            sources.into(),
            filter,
            mode,
            select_fields,
        )
        .map(|iter| iter.into())
    }

    #[pyo3(signature = (query, sources, filter=None, top_k=10, select_fields=None))]
    pub fn search(
        &self,
        py: Python<'_>,
        query: String,
        sources: Sources,
        filter: Option<LogicalExpr>,
        top_k: u32,
        select_fields: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        search(
            self.client.clone(),
            py,
            query,
            sources.into(),
            filter,
            top_k,
            select_fields,
        )
    }

    #[pyo3(signature = (query, sources, filter=None, top_k=10, select_fields=None))]
    pub fn search_stream(
        &self,
        py: Python<'_>,
        query: String,
        sources: Sources,
        filter: Option<LogicalExpr>,
        top_k: u32,
        select_fields: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        search_stream(
            self.client.clone(),
            py,
            query,
            sources.into(),
            filter,
            top_k,
            select_fields,
        )
        .map(|iter| iter.into())
    }
}
