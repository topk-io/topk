use std::{collections::HashMap, sync::Arc};

use pyo3::{exceptions::PyException, prelude::*};

use topk_protos::v1::control::FieldSpec as FieldSpecPb;
use topk_rs::ClientConfig;

use super::error::{CollectionNotFoundError, SchemaValidationError};
use crate::{
    control::{collection::Collection, field_spec::FieldSpec},
    data::{
        query::{ConsistencyLevel, Query},
        value::ValueUnion,
    },
    error::DocumentNotFoundError,
};

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
    pub fn get(&self, collection_name: String) -> PyResult<Collection> {
        let collection = self
            .runtime
            .block_on(self.client.collections().get(&collection_name))
            .map_err(|e| match e {
                topk_rs::Error::CollectionNotFound => {
                    CollectionNotFoundError::new_err(e.to_string())
                }
                _ => PyException::new_err(format!("failed to get collection: {:?}", e)),
            })?;

        Ok(collection.into())
    }

    pub fn list(&self) -> PyResult<Vec<Collection>> {
        let collections = self
            .runtime
            .block_on(self.client.collections().list())
            .map_err(|e| match e {
                _ => PyException::new_err(format!("failed to list collections: {:?}", e)),
            })?;

        Ok(collections.into_iter().map(|i| i.into()).collect())
    }

    pub fn create(
        &self,
        collection_name: String,
        schema: HashMap<String, FieldSpec>,
    ) -> PyResult<Collection> {
        let collection = self
            .runtime
            .block_on(
                self.client.collections().create(
                    &collection_name,
                    schema
                        .into_iter()
                        .map(|(k, v)| (k, v.into()))
                        .collect::<HashMap<String, FieldSpecPb>>(),
                ),
            )
            .map_err(|e| match e {
                topk_rs::Error::SchemaValidationError(e) => {
                    SchemaValidationError::new_err(format!("{:?}", e))
                }
                _ => PyException::new_err(format!("failed to create collection: {:?}", e)),
            })?;

        Ok(collection.into())
    }

    pub fn delete(&self, collection_name: String) -> PyResult<()> {
        Ok(self
            .runtime
            .block_on(self.client.collections().delete(&collection_name))
            .map_err(|e| match e {
                topk_rs::Error::CollectionNotFound => {
                    CollectionNotFoundError::new_err(e.to_string())
                }
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
    #[pyo3(signature = (id, fields=vec![], lsn=None, consistency=None))]
    pub fn get(
        &self,
        id: String,
        fields: Vec<String>,
        lsn: Option<u64>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<HashMap<String, ValueUnion>> {
        let document = self
            .runtime
            .block_on(self.client.collection(&self.collection).get(
                id,
                fields,
                lsn,
                consistency.map(|c| c.into()),
            ))
            .map_err(|e| match e {
                topk_rs::Error::DocumentNotFound => DocumentNotFoundError::new_err(e.to_string()),
                _ => PyException::new_err(format!("failed to get document: {:?}", e)),
            })?;

        Ok(document
            .fields
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect())
    }

    #[pyo3(signature = (lsn=None, consistency=None))]
    pub fn count(&self, lsn: Option<u64>, consistency: Option<ConsistencyLevel>) -> PyResult<u64> {
        let query = Query::new().count()?;

        let docs = self
            .runtime
            .block_on(self.client.collection(&self.collection).query(
                query.into(),
                lsn,
                consistency.map(|c| c.into()),
            ))
            .map_err(|e| match e {
                _ => PyException::new_err(format!("failed to query collection: {:?}", e)),
            })?;

        for doc in docs {
            match doc.fields.get("_count") {
                Some(value) => match value.as_u64() {
                    Some(count) => return Ok(count),
                    None => {
                        return Err(PyException::new_err(format!(
                            "Invalid _count field data type in count query response"
                        )))
                    }
                },
                None => {
                    return Err(PyException::new_err(format!(
                        "Missing _count field in count query response"
                    )))
                }
            }
        }

        Err(PyException::new_err(format!(
            "No documents received for count query"
        )))
    }

    #[pyo3(signature = (query, lsn=None, consistency=None))]
    pub fn query(
        &self,
        query: Query,
        lsn: Option<u64>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<Vec<HashMap<String, ValueUnion>>> {
        let docs = self
            .runtime
            .block_on(self.client.collection(&self.collection).query(
                query.into(),
                lsn,
                consistency.map(|c| c.into()),
            ))
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
