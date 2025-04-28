use super::Channel;
use super::ClientConfig;
use crate::error::ValidationErrorBag;
use crate::error::{Error, InternalErrorCode};
use crate::query::Query;
use crate::query::Stage;
use std::collections::HashMap;
use std::time::Duration;
use topk_protos::utils::{QueryClientWithHeaders, WriteClientWithHeaders};
use topk_protos::v1::data::{ConsistencyLevel, GetRequest};
use topk_protos::{
    utils::{QueryClient, WriteClient},
    v1::data::{DeleteDocumentsRequest, Document, QueryRequest, UpsertDocumentsRequest, Value},
};

#[derive(Clone)]
pub struct CollectionClient {
    config: ClientConfig,
    channel: Channel,
    collection: String,
}

impl CollectionClient {
    pub fn new(config: ClientConfig, channel: Channel, collection: String) -> Self {
        let mut headers = config.headers();
        headers.insert("x-topk-collection", collection.to_string());

        Self {
            config: config.with_headers(headers),
            channel,
            collection,
        }
    }

    pub async fn get(
        &self,
        ids: impl IntoIterator<Item = impl Into<String>>,
        fields: Option<Vec<String>>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<HashMap<String, HashMap<String, Value>>, Error> {
        let mut tries = 0;
        let max_tries = 120;
        let retry_after = Duration::from_secs(1);

        let ids: Vec<String> = ids.into_iter().map(|id| id.into()).collect();

        loop {
            tries += 1;

            let fields = fields.clone();

            let response = self
                .query_client()
                .await?
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
                Err(e) => match InternalErrorCode::parse_status(&e) {
                    // Custom error
                    Ok(InternalErrorCode::RequiredLsnGreaterThanManifestMaxLsn) => {
                        if tries < max_tries {
                            tokio::time::sleep(retry_after).await;
                            continue;
                        } else {
                            return Err(Error::QueryLsnTimeout);
                        }
                    }
                    _ => {
                        return Err(match e.code() {
                            tonic::Code::NotFound => Error::CollectionNotFound,
                            tonic::Code::ResourceExhausted => Error::CapacityExceeded,
                            tonic::Code::InvalidArgument => {
                                Error::InvalidArgument(e.message().into())
                            }
                            _ => Error::Unexpected(e),
                        })
                    }
                },
            };
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
        let mut tries = 0;
        let max_tries = 120;
        let retry_after = Duration::from_secs(1);

        loop {
            tries += 1;

            let query = query.clone();

            let response = self
                .query_client()
                .await?
                .query(QueryRequest {
                    collection: self.collection.clone(),
                    query: Some(query.into()),
                    required_lsn: lsn.clone(),
                    consistency_level: consistency.map(|c| c.into()),
                })
                .await;

            match response {
                Ok(response) => return Ok(response.into_inner().results),
                Err(e) => {
                    match InternalErrorCode::parse_status(&e) {
                        // Custom error
                        Ok(InternalErrorCode::RequiredLsnGreaterThanManifestMaxLsn) => {
                            if tries < max_tries {
                                tokio::time::sleep(retry_after).await;
                                continue;
                            } else {
                                return Err(Error::QueryLsnTimeout);
                            }
                        }
                        _ => {
                            return Err(match e.code() {
                                tonic::Code::NotFound => Error::CollectionNotFound,
                                tonic::Code::ResourceExhausted => Error::CapacityExceeded,
                                tonic::Code::InvalidArgument => {
                                    Error::InvalidArgument(e.message().into())
                                }
                                _ => Error::Unexpected(e),
                            })
                        }
                    }
                }
            }
        }
    }

    pub async fn upsert(&self, docs: Vec<Document>) -> Result<String, Error> {
        let mut client = self.write_client().await?;

        let response = client
            .upsert_documents(UpsertDocumentsRequest { docs })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::NotFound => Error::CollectionNotFound,
                tonic::Code::InvalidArgument => match ValidationErrorBag::try_from(e.clone()) {
                    Ok(errors) => Error::DocumentValidationError(errors),
                    Err(_) => Error::Unexpected(e),
                },
                tonic::Code::ResourceExhausted => Error::CapacityExceeded,
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().lsn)
    }

    pub async fn delete(&self, ids: Vec<String>) -> Result<String, Error> {
        let mut client = self.write_client().await?;

        let response = client
            .delete_documents(DeleteDocumentsRequest { ids })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::NotFound => Error::CollectionNotFound,
                tonic::Code::ResourceExhausted => Error::CapacityExceeded,
                tonic::Code::InvalidArgument => match ValidationErrorBag::try_from(e.clone()) {
                    Ok(errors) => Error::DocumentValidationError(errors),
                    Err(_) => Error::Unexpected(e),
                },
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().lsn)
    }

    async fn write_client(&self) -> Result<WriteClientWithHeaders, Error> {
        Ok(WriteClient::with_headers(
            self.channel.get().await?,
            self.config.headers(),
        ))
    }

    async fn query_client(&self) -> Result<QueryClientWithHeaders, Error> {
        Ok(QueryClient::with_headers(
            self.channel.get().await?,
            self.config.headers(),
        ))
    }
}
