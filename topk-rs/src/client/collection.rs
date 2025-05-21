use super::client_config::ClientConfig;
use super::create_query_client;
use super::create_write_client;
use super::QueryChannel;
use super::WriterChannel;
use crate::error::Error;
use crate::query::Query;
use crate::query::Stage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
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
    writer_channel: Arc<WriterChannel>,
    query_channel: Arc<QueryChannel>,
}

impl CollectionClient {
    pub fn new(
        config: Arc<ClientConfig>,
        writer_channel: Arc<WriterChannel>,
        query_channel: Arc<QueryChannel>,
        collection_name: String,
    ) -> Self {
        Self {
            config,
            writer_channel,
            query_channel,
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
        let mut client =
            create_query_client(&self.config, &self.collection_name, &self.query_channel).await?;

        let mut tries = 0;
        let max_tries = 120;
        let retry_after = Duration::from_secs(1);

        let ids: Vec<String> = ids.into_iter().map(|id| id.into()).collect();

        loop {
            tries += 1;

            let fields = fields.clone();

            let response = client
                .get(GetRequest {
                    ids: ids.clone(),
                    fields: fields.unwrap_or_default(),
                    required_lsn: lsn.clone(),
                    consistency_level: consistency.map(|c| c.into()),
                })
                .await;

            match response {
                Ok(response) => {
                    return Ok(response
                        .into_inner()
                        .docs
                        .into_iter()
                        .map(|(id, doc)| (id, doc.fields))
                        .collect())
                }
                Err(e) => match e.code() {
                    tonic::Code::NotFound => return Err(Error::CollectionNotFound),
                    _ => match e.into() {
                        Error::QueryLsnTimeout => {
                            if tries < max_tries {
                                tokio::time::sleep(retry_after).await;
                                continue;
                            } else {
                                return Err(Error::QueryLsnTimeout);
                            }
                        }
                        e => return Err(e),
                    },
                },
            }
        }
    }

    pub async fn count(
        &self,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<u64, Error> {
        let query = Query::new(vec![Stage::Count {}]);

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
        // Initialize the client
        let mut client =
            create_query_client(&self.config, &self.collection_name, &self.query_channel).await?;

        // Retry logic
        // TODO: refactor to use a retry policy
        let mut tries = 0;
        let max_tries = 120;
        let retry_after = Duration::from_secs(1);

        loop {
            tries += 1;

            let query = query.clone();

            let response = client
                .query(QueryRequest {
                    collection: self.collection_name.clone(),
                    query: Some(query.into()),
                    required_lsn: lsn.clone(),
                    consistency_level: consistency.map(|c| c.into()),
                })
                .await;

            match response {
                Ok(response) => return Ok(response.into_inner().results),
                Err(e) => match e.code() {
                    // Explicitly map `NotFound` to `CollectionNotFound` error
                    tonic::Code::NotFound => return Err(Error::CollectionNotFound),
                    // Delegate other errors
                    _ => match e.into() {
                        Error::QueryLsnTimeout => {
                            if tries < max_tries {
                                tokio::time::sleep(retry_after).await;
                                continue;
                            } else {
                                return Err(Error::QueryLsnTimeout);
                            }
                        }
                        e => return Err(e),
                    },
                },
            }
        }
    }

    pub async fn upsert(&self, docs: Vec<Document>) -> Result<String, Error> {
        let mut client =
            create_write_client(&self.config, &self.collection_name, &self.writer_channel).await?;

        let response = client
            .upsert_documents(UpsertDocumentsRequest { docs })
            .await
            .map_err(|e| match e.code() {
                // Explicitly map `NotFound` to `CollectionNotFound` error
                tonic::Code::NotFound => Error::CollectionNotFound,
                // Delegate other errors
                _ => e.into(),
            })?;

        Ok(response.into_inner().lsn)
    }

    pub async fn delete(&self, ids: Vec<String>) -> Result<String, Error> {
        let mut client =
            create_write_client(&self.config, &self.collection_name, &self.writer_channel).await?;

        let response = client
            .delete_documents(DeleteDocumentsRequest { ids })
            .await
            .map_err(|e| match e.code() {
                // Explicitly map `NotFound` to `CollectionNotFound` error
                tonic::Code::NotFound => Error::CollectionNotFound,
                // Delegate other errors
                _ => e.into(),
            })?;

        Ok(response.into_inner().lsn)
    }
}
