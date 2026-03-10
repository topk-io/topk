use std::collections::HashMap;
use std::pin::Pin;

use futures::{Stream, TryStreamExt};
use futures_util::{StreamExt, TryFutureExt};
use prost::Message;
use std::sync::Arc;

use tokio::sync::OnceCell;
use tonic::transport::Channel;

use crate::create_client;
use crate::error::Error;
use crate::proto::v1::data::query_service_client::QueryServiceClient;
use crate::proto::v1::data::write_service_client::WriteServiceClient;
use crate::proto::v1::data::Query;
use crate::proto::v1::data::Stage;
use crate::proto::v1::data::UpdateDocumentsRequest;
use crate::proto::v1::data::{ConsistencyLevel, GetRequest};
use crate::proto::v1::data::{
    DeleteDocumentsRequest, Document, QueryRequest, UpsertDocumentsRequest, Value,
};

use super::config::ClientConfig;
use super::retry::call_with_retry;

#[derive(Clone)]
pub struct CollectionClient {
    // Client config
    config: ClientConfig,

    // Read channel
    read: Arc<OnceCell<Channel>>,

    // Write channel
    write: Arc<OnceCell<Channel>>,
}

impl CollectionClient {
    pub fn new(
        config: ClientConfig,
        read: Arc<OnceCell<Channel>>,
        write: Arc<OnceCell<Channel>>,
    ) -> Self {
        Self {
            config,
            read,
            write,
        }
    }

    pub async fn get(
        &self,
        ids: impl IntoIterator<Item = impl Into<String>>,
        fields: Option<Vec<String>>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<HashMap<String, HashMap<String, Value>>, Error> {
        let client = create_client!(QueryServiceClient, self.read, self.config).await?;
        let ids: Vec<String> = ids.into_iter().map(|id| id.into()).collect();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let ids = ids.clone();
            let fields = fields.clone();
            let lsn = lsn.clone();
            let consistency = consistency.clone();

            async move {
                client
                    .get_stream(GetRequest {
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

        // Collect results from stream
        let mut stream = response.into_inner();
        let mut docs = HashMap::new();
        while let Some(result) = stream.next().await {
            // Decode document
            let doc = Document::decode(result?.data)
                .map_err(|e| Error::MalformedResponse(e.to_string()))?;

            docs.insert(
                doc.id()
                    .map_err(|e| Error::MalformedResponse(e.to_string()))?
                    .to_string(),
                doc.fields,
            );
        }
        Ok(docs)
    }

    pub async fn count(
        &self,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<u64, Error> {
        let query = Query::new(vec![Stage::count()]);

        let docs = call_with_retry(&self.config.retry_config(), || {
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
        let stream = self.query_stream(query, lsn, consistency).await?;
        let results = stream.try_collect::<Vec<_>>().await?;
        Ok(results)
    }

    pub async fn query_stream(
        &self,
        query: Query,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Document, Error>> + Send>>, Error> {
        let client = create_client!(QueryServiceClient, self.read, self.config).await?;

        let stream = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let query = query.clone();
            let lsn = lsn.clone();

            async move {
                client
                    .query_stream(QueryRequest {
                        query: Some(query.into()),
                        required_lsn: lsn.clone(),
                        consistency_level: consistency.map(|c| c.into()),
                        // DEPRECATED: This field is no longer used, kept for backwards compatibility.
                        collection: String::new(),
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
        .await?
        .into_inner();

        let stream = stream.map(|result| match Document::decode(result?.data) {
            Ok(doc) => Ok(doc),
            Err(e) => Err(Error::MalformedResponse(e.to_string())),
        });

        Ok(Box::pin(stream))
    }

    /// Upsert documents into the collection.
    ///
    /// Existing documents will be replaced, new documents will be created.
    pub async fn upsert(&self, docs: Vec<Document>) -> Result<String, Error> {
        let client = create_client!(WriteServiceClient, self.write, self.config).await?;

        let response = call_with_retry(&self.config.retry_config(), || {
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

    /// Update documents in the collection.
    ///
    /// Existing documents will be merged with the provided fields.
    /// Missing documents will be ignored.
    pub async fn update(
        &self,
        docs: Vec<Document>,
        fail_on_missing: bool,
    ) -> Result<String, Error> {
        let client = create_client!(WriteServiceClient, self.write, self.config).await?;

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let docs = docs.clone();

            async move {
                client
                    .update_documents(UpdateDocumentsRequest {
                        docs,
                        fail_on_missing,
                    })
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

    /// Delete documents from the collection.
    pub async fn delete(&self, req: impl Into<DeleteDocumentsRequest>) -> Result<String, Error> {
        let client = create_client!(WriteServiceClient, self.write, self.config).await?;

        let req = req.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let req = req.clone();

            async move {
                client
                    .delete_documents(req)
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
