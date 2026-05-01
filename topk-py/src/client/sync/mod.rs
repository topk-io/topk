mod ask;
mod collection;
mod collections;
mod dataset;
mod datasets;
mod runtime;
mod search;

use std::sync::Arc;

pub use ask::{ask, AskIterator};
pub use collection::CollectionClient;
pub use collections::CollectionsClient;
pub use dataset::DatasetClient;
pub use dataset::DatasetListIterator;
pub use datasets::DatasetsClient;
pub use search::{search, SearchIterator};

use crate::{
    client::{sync::runtime::Runtime, topk_client, NativeRetryConfig},
    data::ask::{Mode, Source},
    expr::logical::LogicalExpr,
};

use pyo3::{pyclass, pymethods, PyResult};

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

    #[pyo3(signature = (query, datasets, filter=None, mode=None, select_fields=None))]
    pub fn ask(
        &self,
        query: String,
        datasets: Vec<Source>,
        filter: Option<LogicalExpr>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> PyResult<AskIterator> {
        ask(
            self.runtime.clone(),
            self.client.clone(),
            query,
            datasets,
            filter,
            mode,
            select_fields,
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
    ) -> PyResult<SearchIterator> {
        search(
            self.runtime.clone(),
            self.client.clone(),
            query,
            datasets,
            filter,
            top_k,
            select_fields,
        )
    }
}
