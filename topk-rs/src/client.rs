use crate::{Error, InternalErrorCode};
use std::str::FromStr;
use std::{collections::HashMap, time::Duration};
use tokio::sync::OnceCell;
use tonic::transport::{Channel, Endpoint};
use topk_protos::utils::{
    DocumentClientWithHeaders, IndexClient, IndexClientWithHeaders, QueryClientWithHeaders,
};
use topk_protos::v1::control::doc_validation::ValidationErrorBag;
use topk_protos::{
    utils::{DocumentClient, QueryClient},
    v1::{
        control::{
            index_schema::IndexSchema, CreateIndexRequest, DeleteIndexRequest, Index,
            ListIndexesRequest,
        },
        data::{DeleteDocumentsRequest, Document, Query, QueryRequest, UpsertDocumentsRequest},
    },
};

#[derive(Clone)]
pub struct ClientConfig {
    api_key: String,
    region: String,
    host: String,
    https: bool,
}

impl ClientConfig {
    pub fn new(api_key: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            region: region.into(),
            host: "api.topk.io".to_string(),
            https: true,
        }
    }

    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn with_https(mut self, https: bool) -> Self {
        self.https = https;
        self
    }

    pub fn headers(&self) -> HashMap<&'static str, String> {
        HashMap::from([("authorization", format!("Bearer {}", self.api_key))])
    }

    pub fn endpoint(&self) -> String {
        let protocol = if self.https { "https" } else { "http" };

        format!("{}://{}.api.{}", protocol, self.region, self.host)
    }
}

#[derive(Clone)]
pub struct Client {
    config: ClientConfig,
    channel: OnceCell<Channel>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            channel: OnceCell::new(),
        }
    }

    // Data plane APIs

    pub async fn query(&self, index: &str, query: Query) -> Result<Vec<Document>, Error> {
        self.query_at_lsn(index, query, None).await
    }

    pub async fn query_at_lsn(
        &self,
        index: &str,
        query: Query,
        lsn: Option<u64>,
    ) -> Result<Vec<Document>, Error> {
        let mut tries = 0;
        let max_tries = 10;
        let retry_after = Duration::from_secs(1);

        loop {
            tries += 1;

            let query = query.clone();

            let response = self
                .query_client(index)
                .await?
                .query(QueryRequest {
                    collection: index.to_string(),
                    query: Some(query),
                    required_lsn: lsn,
                })
                .await;

            match response {
                Ok(response) => return Ok(response.into_inner().results),
                Err(e) => {
                    let code = InternalErrorCode::parse_status(&e);

                    match code {
                        Ok(InternalErrorCode::RequiredLsnGreaterThanManifestMaxLsn) => {
                            if tries < max_tries {
                                tokio::time::sleep(retry_after).await;

                                continue;
                            } else {
                                return Err(Error::QueryLsnTimeout);
                            }
                        }
                        _ => return Err(Error::Unexpected(e)),
                    }
                }
            }
        }
    }

    pub async fn upsert(&self, index: &str, docs: Vec<Document>) -> Result<u64, Error> {
        let mut client = self.doc_client(index).await?;

        let response = client
            .upsert_documents(UpsertDocumentsRequest { docs })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::InvalidArgument => match ValidationErrorBag::try_from(e.clone()) {
                    Ok(errors) => Error::DocumentValidationError(errors),
                    Err(_) => Error::Unexpected(e),
                },
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().lsn)
    }

    pub async fn delete(&self, index: &str, ids: Vec<String>) -> Result<u64, Error> {
        let mut client = self.doc_client(index).await?;

        let response = client
            .delete_documents(DeleteDocumentsRequest { ids })
            .await
            .map_err(|e| match e.code() {
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().lsn)
    }

    // Control plane APIs

    pub async fn list_indexes(&self) -> Result<Vec<Index>, Error> {
        let response = self
            .index_client()
            .await?
            .list_indexes(ListIndexesRequest {})
            .await
            .map_err(|e| match e.code() {
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().indexes)
    }

    pub async fn create_index(
        &self,
        name: impl Into<String>,
        schema: IndexSchema,
    ) -> Result<Index, Error> {
        let response = self
            .index_client()
            .await?
            .create_index(CreateIndexRequest {
                name: name.into(),
                schema: schema.into_fields(),
            })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::AlreadyExists => Error::IndexAlreadyExists,
                tonic::Code::InvalidArgument => match ValidationErrorBag::try_from(e.clone()) {
                    Ok(errors) => Error::SchemaValidationError(errors),
                    Err(_) => Error::Unexpected(e),
                },
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().index.expect("invalid proto"))
    }

    pub async fn delete_index(&self, name: impl Into<String>) -> Result<(), Error> {
        self.index_client()
            .await?
            .delete_index(DeleteIndexRequest { name: name.into() })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::NotFound => Error::IndexNotFound,
                _ => Error::Unexpected(e),
            })?;

        Ok(())
    }

    // Private methods

    async fn doc_client(&self, index: &str) -> Result<DocumentClientWithHeaders, Error> {
        let mut headers = self.config.headers();
        headers.insert("x-topk-collection", index.to_string());

        Ok(DocumentClient::with_headers(self.channel().await?, headers))
    }

    async fn query_client(&self, index: &str) -> Result<QueryClientWithHeaders, Error> {
        let mut headers = self.config.headers();
        headers.insert("x-topk-collection", index.to_string());

        Ok(QueryClient::with_headers(self.channel().await?, headers))
    }

    async fn index_client(&self) -> Result<IndexClientWithHeaders, Error> {
        Ok(IndexClient::with_headers(
            self.channel().await?,
            self.config.headers(),
        ))
    }

    async fn channel(&self) -> Result<Channel, Error> {
        let channel = self
            .channel
            .get_or_try_init(|| async {
                Endpoint::from_str(&self.config.endpoint())?.connect().await
            })
            .await?;

        Ok(channel.clone())
    }
}
