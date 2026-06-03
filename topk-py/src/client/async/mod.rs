use std::sync::Arc;

use pyo3::{prelude::*, PyResult};

use crate::{
    client::{
        r#async::ask::ask,
        r#async::search::search,
        topk_client, NativeRetryConfig,
    },
    data::ask::{Mode, Source},
    expr::logical::LogicalExpr,
};

mod ask;
mod collection;
mod collections;
mod dataset;
mod datasets;
mod search;

pub use ask::AsyncAskIterator;
pub use collection::{AsyncCollectionClient, AsyncPartitionListIterator};
pub use collections::AsyncCollectionsClient;
pub use dataset::AsyncDatasetClient;
pub use dataset::AsyncDatasetListIterator;
pub use datasets::AsyncDatasetsClient;
pub use search::AsyncSearchIterator;

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

    #[pyo3(signature = (collection, partition=None))]
    pub fn collection(
        &self,
        collection: String,
        partition: Option<String>,
    ) -> PyResult<AsyncCollectionClient> {
        Ok(AsyncCollectionClient::new(
            self.client.clone(),
            Arc::new(collection),
            partition,
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

    #[pyo3(
        signature = (query, datasets, filter=None, mode=None, select_fields=None, include_content=None)
    )]
    pub fn ask(
        &self,
        query: String,
        datasets: Vec<Source>,
        filter: Option<LogicalExpr>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
        include_content: Option<bool>,
    ) -> PyResult<AsyncAskIterator> {
        ask(
            self.client.clone(),
            query,
            datasets,
            filter,
            mode,
            select_fields,
            include_content,
        )
    }

    #[pyo3(signature = (query, datasets, top_k, filter=None, select_fields=None))]
    pub fn search(
        &self,
        query: String,
        datasets: Vec<Source>,
        top_k: u32,
        filter: Option<LogicalExpr>,
        select_fields: Option<Vec<String>>,
    ) -> PyResult<AsyncSearchIterator> {
        search(
            self.client.clone(),
            query,
            datasets,
            filter,
            top_k,
            select_fields,
        )
    }
}
