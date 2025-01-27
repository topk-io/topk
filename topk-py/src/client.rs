use pyo3::{exceptions::PyException, prelude::*};
use std::{collections::HashMap, sync::Arc};
use topk_protos::v1::control::index_schema::IndexSchema;
use topk_rs::ClientConfig;

use crate::{
    control::{collection::Collection, field_spec::FieldSpec},
    data::{query::Query, value::ValueUnion},
};

use super::error::{CollectionNotFoundError, SchemaValidationError};

#[pyclass]
pub struct Client {
    runtime: Arc<tokio::runtime::Runtime>,
    client: Arc<topk_rs::Client>,
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (api_key, region, host="topk.io".into(), https=true))]
    pub fn new(api_key: String, region: String, host: String, https: bool) -> Self {
        let runtime =
            Arc::new(tokio::runtime::Runtime::new().expect("failed to create tokio runtime"));
        let client = Arc::new(topk_rs::Client::new(
            ClientConfig::new(api_key, region)
                .with_https(https)
                .with_host(host),
        ));

        Self { runtime, client }
    }

    pub fn collection(&self, collection: String) -> PyResult<CollectionClient> {
        Ok(CollectionClient {
            runtime: self.runtime.clone(),
            client: self.client.clone(),
            collection,
        })
    }

    pub fn collections(&self) -> PyResult<CollectionsClient> {
        Ok(CollectionsClient {
            runtime: self.runtime.clone(),
            client: self.client.clone(),
        })
    }
}

#[pyclass]
pub struct CollectionsClient {
    runtime: Arc<tokio::runtime::Runtime>,
    client: Arc<topk_rs::Client>,
}

#[pymethods]
impl CollectionsClient {
    pub fn list(&self) -> PyResult<Vec<Collection>> {
        let indexes = self
            .runtime
            .block_on(self.client.collections().list())
            .map_err(|e| match e {
                _ => PyException::new_err(format!("failed to list collections: {:?}", e)),
            })?;

        Ok(indexes.into_iter().map(|i| i.into()).collect())
    }

    pub fn create(
        &self,
        index_name: String,
        schema: HashMap<String, FieldSpec>,
    ) -> PyResult<Collection> {
        let index = self
            .runtime
            .block_on(self.client.collections().create(
                &index_name,
                IndexSchema::new(schema.into_iter().map(|(k, v)| (k, v.into())).collect()),
            ))
            .map_err(|e| match e {
                topk_rs::Error::SchemaValidationError(e) => {
                    SchemaValidationError::new_err(format!("{:?}", e))
                }
                _ => PyException::new_err(format!("failed to create collection: {:?}", e)),
            })?;

        Ok(index.into())
    }

    pub fn delete(&self, index_name: String) -> PyResult<()> {
        Ok(self
            .runtime
            .block_on(self.client.collections().delete(&index_name))
            .map_err(|e| match e {
                topk_rs::Error::IndexNotFound => CollectionNotFoundError::new_err(e.to_string()),
                _ => PyException::new_err(format!("failed to delete collection: {:?}", e)),
            })?)
    }
}

#[pyclass]
pub struct CollectionClient {
    runtime: Arc<tokio::runtime::Runtime>,
    client: Arc<topk_rs::Client>,
    collection: String,
}

#[pymethods]
impl CollectionClient {
    #[pyo3(signature = (query, lsn=None))]
    pub fn query(
        &self,
        query: Query,
        lsn: Option<u64>,
    ) -> PyResult<Vec<HashMap<String, ValueUnion>>> {
        let docs = self
            .runtime
            .block_on(
                self.client
                    .collection(&self.collection)
                    .query_at_lsn(query.into(), lsn),
            )
            .map_err(|e| match e {
                _ => PyException::new_err(format!("failed to query collection: {:?}", e)),
            })?;

        Ok(docs
            .into_iter()
            .map(|d| d.fields.into_iter().map(|(k, v)| (k, v.into())).collect())
            .collect())
    }

    pub fn upsert(&self, documents: Vec<HashMap<String, ValueUnion>>) -> PyResult<u64> {
        let documents = documents
            .into_iter()
            .map(|d| topk_protos::v1::data::Document {
                fields: d.into_iter().map(|(k, v)| (k, v.into())).collect(),
            })
            .collect();

        Ok(self
            .runtime
            .block_on(self.client.collection(&self.collection).upsert(documents))
            .map_err(|e| match e {
                _ => PyException::new_err(format!("failed to upsert documents: {:?}", e)),
            })?)
    }

    pub fn delete(&self, ids: Vec<String>) -> PyResult<u64> {
        Ok(self
            .runtime
            .block_on(self.client.collection(&self.collection).delete(ids))
            .map_err(|e| match e {
                _ => PyException::new_err(format!("failed to delete documents: {:?}", e)),
            })?)
    }
}
