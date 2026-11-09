use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::{Stream, TryStreamExt};
use futures_util::StreamExt;
use prost::Message;
use tokio::sync::OnceCell;
use tonic::transport::Channel;
use tonic::{Response, Streaming};

use super::retry::call_with_retry;
use crate::create_client;
use crate::error::Error;
use crate::proto::v1::data::partition_service_client::PartitionServiceClient;
use crate::proto::v1::data::query_service_client::QueryServiceClient;
use crate::proto::v1::data::write_service_client::WriteServiceClient;
use crate::proto::v1::data::ListPartitionsRequest;
use crate::proto::v1::data::Partition;
use crate::proto::v1::data::Query;
use crate::proto::v1::data::Stage;
use crate::proto::v1::data::UpdateDocumentsRequest;
use crate::proto::v1::data::{ConsistencyLevel, GetRequest};
use crate::proto::v1::data::{
    DeleteDocumentsRequest, Document, QueryRequest, UpsertDocumentsRequest, Value,
};
use crate::proto::v1::data::{DeletePartitionRequest, DocumentData};

use super::config::ClientConfig;

pub struct DocumentStream {
    // Underying stream
    stream: Pin<Box<dyn Stream<Item = Result<Document, Error>> + Send>>,
    // Number of matched candidate documents
    matched_count: Option<u64>,
}

impl DocumentStream {
    pub fn new(response: Response<Streaming<DocumentData>>) -> Self {
        let matched_count = response
            .metadata()
            .get("x-topk-matched-count")
            .and_then(|v| v.to_str().ok()?.parse().ok());

        let stream = response
            .into_inner()
            .map_err(CollectionClient::map_status_to_error)
            .map(|result| match Document::decode(result?.data) {
                Ok(doc) => Ok(doc),
                Err(e) => Err(Error::MalformedResponse(e.to_string())),
            });

        Self {
            stream: Box::pin(stream),
            matched_count,
        }
    }

    pub fn matched_count(&self) -> Option<u64> {
        self.matched_count
    }
}

impl Stream for DocumentStream {
    type Item = Result<Document, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx)
    }
}

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

    /// Sets a partition for this [`CollectionClient`].
    pub fn partition(mut self, partition_name: impl Into<String>) -> CollectionClient {
        self.config = self
            .config
            .with_headers([("x-topk-partition", partition_name)]);

        self
    }

    pub async fn get(
        &self,
        ids: impl IntoIterator<Item = impl Into<String>>,
        fields: Option<Vec<String>>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<HashMap<String, HashMap<String, Value>>, Error> {
        let client = create_client!(QueryServiceClient, self.read, self.config).await?;

        let request = GetRequest {
            ids: ids.into_iter().map(|id| id.into()).collect(),
            fields: fields.unwrap_or_default(),
            // A no-op write returns an empty LSN, which means there is nothing to wait for.
            required_lsn: lsn.filter(|lsn| !lsn.is_empty()),
            consistency_level: consistency.map(|c| c.into()),
        };

        let response = call_with_retry(self.config.retry_config(), || {
            let mut client = client.clone();
            let request = request.clone();

            async move {
                client
                    .get_stream(request)
                    .await
                    .map_err(Self::map_status_to_error)
            }
        })
        .await?;

        // Collect results from stream
        let mut stream = response.into_inner().map_err(Self::map_status_to_error);
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

        let docs = self.query(query, lsn, consistency).await?;

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
    ) -> Result<DocumentStream, Error> {
        let client = create_client!(QueryServiceClient, self.read, self.config).await?;

        let request = QueryRequest {
            query: Some(query.into()),
            // A no-op write returns an empty LSN, which means there is nothing to wait for.
            required_lsn: lsn.filter(|lsn| !lsn.is_empty()),
            consistency_level: consistency.map(|c| c.into()),
            ..Default::default()
        };

        let response = call_with_retry(self.config.retry_config(), || {
            let mut client = client.clone();
            let request = request.clone();

            async move {
                client
                    .query_stream(request)
                    .await
                    .map_err(Self::map_status_to_error)
            }
        })
        .await?;

        Ok(DocumentStream::new(response))
    }

    /// Upsert documents into the collection.
    ///
    /// Existing documents will be replaced, new documents will be created.
    pub async fn upsert(&self, docs: Vec<Document>) -> Result<String, Error> {
        let client = create_client!(WriteServiceClient, self.write, self.config).await?;

        let request = UpsertDocumentsRequest { docs };

        let response = call_with_retry(self.config.retry_config(), || {
            let mut client = client.clone();
            let request = request.clone();

            async move {
                client
                    .upsert_documents(request)
                    .await
                    .map_err(Self::map_status_to_error)
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

        let request = UpdateDocumentsRequest {
            docs,
            fail_on_missing,
        };

        let response = call_with_retry(self.config.retry_config(), || {
            let mut client = client.clone();
            let request = request.clone();

            async move {
                client
                    .update_documents(request)
                    .await
                    .map_err(Self::map_status_to_error)
            }
        })
        .await?;

        Ok(response.into_inner().lsn)
    }

    /// Delete documents from the collection.
    pub async fn delete(&self, req: impl Into<DeleteDocumentsRequest>) -> Result<String, Error> {
        let client = create_client!(WriteServiceClient, self.write, self.config).await?;

        let request: DeleteDocumentsRequest = req.into();

        let response = call_with_retry(self.config.retry_config(), || {
            let mut client = client.clone();
            let request = request.clone();

            async move {
                client
                    .delete_documents(request)
                    .await
                    .map_err(Self::map_status_to_error)
            }
        })
        .await?;

        Ok(response.into_inner().lsn)
    }

    /// List partitions in the collection.
    ///
    /// If `prefix` is provided, only partitions with names starting with the prefix will be listed.
    /// Returns a stream of partitions matching the prefix (if provided).
    pub async fn list_partitions(
        &self,
        prefix: Option<String>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Partition, Error>> + Send>>, Error> {
        let client = create_client!(PartitionServiceClient, self.write, self.config).await?;

        let request = ListPartitionsRequest { prefix };

        let stream = call_with_retry(self.config.retry_config(), || {
            let mut client = client.clone();
            let request = request.clone();

            async move {
                client
                    .list(request)
                    .await
                    .map_err(Self::map_status_to_error)
            }
        })
        .await?
        .into_inner();

        Ok(Box::pin(stream.map_err(Self::map_status_to_error)))
    }

    /// Delete partition from the collection.
    ///
    /// All documents in the partition will be deleted atomically.
    pub async fn delete_partition(&self, name: impl Into<String>) -> Result<(), Error> {
        let client = create_client!(PartitionServiceClient, self.write, self.config).await?;

        let request = DeletePartitionRequest { name: name.into() };

        call_with_retry(self.config.retry_config(), || {
            let mut client = client.clone();
            let request = request.clone();

            async move {
                client
                    .delete(request)
                    .await
                    .map_err(Self::map_status_to_error)
            }
        })
        .await?;

        Ok(())
    }

    #[inline]
    fn map_status_to_error(status: tonic::Status) -> Error {
        match Error::from(status) {
            // Explicitly map `NotFound` to `CollectionNotFound` error
            Error::NotFound => Error::CollectionNotFound,
            // Pass through other errors
            err => err,
        }
    }
}
