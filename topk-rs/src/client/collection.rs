use super::config::ClientConfig;
use super::create_query_client;
use super::create_write_client;
use super::retry::call_with_retry;
use crate::error::Error;
use crate::query::Query;
use crate::query::Stage;
use futures_util::future::TryFutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tonic::transport::Channel;
use topk_protos::v1::data::{ConsistencyLevel, GetRequest};
use topk_protos::v1::data::{
    DeleteDocumentsRequest, Document, QueryRequest, UpsertDocumentsRequest, Value,
};

#[derive(Clone)]
pub struct CollectionClient {
    // Client config
    config: Arc<ClientConfig>,

    // Collection name
    collection_name: String,

    // Channels
    channel: Arc<OnceCell<Channel>>,
}

impl CollectionClient {
    pub fn new(
        config: Arc<ClientConfig>,
        channel: Arc<OnceCell<Channel>>,
        collection_name: String,
    ) -> Self {
        Self {
            config,
            channel,
            collection_name,
        }
    }

    pub async fn get(
        &self,
        ids: impl IntoIterator<Item = impl Into<String>>,
        fields: Option<Vec<String>>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<HashMap<String, HashMap<String, Value>>, Error> {
        let client =
            create_query_client(&self.config, &self.collection_name, &self.channel).await?;
        let ids: Vec<String> = ids.into_iter().map(|id| id.into()).collect();

        let response = call_with_retry(&self.config.retry_config, || {
            let mut client = client.clone();
            let ids = ids.clone();
            let fields = fields.clone();
            let lsn = lsn.clone();
            let consistency = consistency.clone();

            async move {
                client
                    .get(GetRequest {
                        ids: ids,
                        fields: fields.unwrap_or_default(),
                        required_lsn: lsn,
                        consistency_level: consistency.map(|c| c.into()),
                    })
                    .map_err(|e| match e.code() {
                        // Collection not found
                        tonic::Code::NotFound => Error::CollectionNotFound,
                        // Delegate other errors
                        _ => e.into(),
                    })
                    .await
            }
        })
        .await?;

        Ok(response
            .into_inner()
            .docs
            .into_iter()
            .map(|(id, doc)| (id, doc.fields))
            .collect())
    }

    pub async fn count(
        &self,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<u64, Error> {
        let query = Query::new(vec![Stage::Count {}]);

        let docs = call_with_retry(&self.config.retry_config, || {
            let query = query.clone();
            let lsn = lsn.clone();
            let consistency = consistency.clone();

            async move { self.query(query, lsn, consistency).await }
        })
        .await?;

        for doc in docs {
            match doc.fields.get("_count") {
                Some(value) => match value.as_u64() {
                    Some(count) => return Ok(count),
                    None => {
                        return Err(Error::MalformedResponse(format!(
                            "Invalid _count field data type in count query response"
                        )))
                    }
                },
                None => {
                    return Err(Error::MalformedResponse(format!(
                        "Missing _count field in count query response"
                    )))
                }
            }
        }

        Err(Error::MalformedResponse(format!(
            "No documents received for count query"
        )))
    }

    pub async fn query(
        &self,
        query: Query,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<Vec<Document>, Error> {
        let client =
            create_query_client(&self.config, &self.collection_name, &self.channel).await?;

        let response = call_with_retry(&self.config.retry_config, || {
            let mut client = client.clone();
            let query = query.clone();
            let lsn = lsn.clone();

            async move {
                client
                    .query(QueryRequest {
                        collection: self.collection_name.clone(),
                        query: Some(query.into()),
                        required_lsn: lsn.clone(),
                        consistency_level: consistency.map(|c| c.into()),
                    })
                    .map_err(|e| match e.code() {
                        // Explicitly map `NotFound` to `CollectionNotFound` error
                        tonic::Code::NotFound => Error::CollectionNotFound,
                        // Delegate other errors
                        _ => e.into(),
                    })
                    .await
            }
        })
        .await?;

        Ok(response.into_inner().results)
    }

    pub async fn upsert(&self, docs: Vec<Document>) -> Result<String, Error> {
        let client =
            create_write_client(&self.config, &self.collection_name, &self.channel).await?;

        let response = call_with_retry(&self.config.retry_config, || {
            let mut client = client.clone();
            let docs = docs.clone();

            async move {
                client
                    .upsert_documents(UpsertDocumentsRequest { docs })
                    .await
                    .map_err(|e| match e.code() {
                        // Explicitly map `NotFound` to `CollectionNotFound` error
                        tonic::Code::NotFound => Error::CollectionNotFound,
                        // Delegate other errors
                        _ => e.into(),
                    })
            }
        })
        .await?;

        Ok(response.into_inner().lsn)
    }

    pub async fn delete(&self, ids: Vec<String>) -> Result<String, Error> {
        let client =
            create_write_client(&self.config, &self.collection_name, &self.channel).await?;

        let response = call_with_retry(&self.config.retry_config, || {
            let mut client = client.clone();
            let ids = ids.clone();

            async move {
                client
                    .delete_documents(DeleteDocumentsRequest { ids })
                    .await
                    .map_err(|e| match e.code() {
                        // Explicitly map `NotFound` to `CollectionNotFound` error
                        tonic::Code::NotFound => Error::CollectionNotFound,
                        // Delegate other errors
                        _ => e.into(),
                    })
            }
        })
        .await?;

        Ok(response.into_inner().lsn)
    }
}
