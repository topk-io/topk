use crate::{Error, InternalErrorCode};
use std::str::FromStr;
use std::{collections::HashMap, time::Duration};
use tokio::sync::OnceCell;
use tonic::transport::{Channel as TonicChannel, Endpoint};
use topk_protos::utils::{
    CollectionClient as ProtoCollectionClient, CollectionClientWithHeaders,
    DocumentClientWithHeaders, QueryClientWithHeaders,
};
use topk_protos::v1::control::doc_validation::ValidationErrorBag;
use topk_protos::v1::control::GetCollectionRequest;
use topk_protos::{
    utils::{DocumentClient, QueryClient},
    v1::{
        control::{
            collection_schema::CollectionSchema, Collection, CreateCollectionRequest,
            DeleteCollectionRequest, ListCollectionsRequest,
        },
        data::{DeleteDocumentsRequest, Document, Query, QueryRequest, UpsertDocumentsRequest},
    },
};

#[derive(Clone)]
pub enum Channel {
    Endpoint(String),
    Tonic(OnceCell<TonicChannel>),
}

impl Channel {
    pub fn from_endpoint(endpoint: impl Into<String>) -> Self {
        Self::Endpoint(endpoint.into())
    }

    pub fn from_tonic(channel: TonicChannel) -> Self {
        Self::Tonic(OnceCell::from(channel))
    }

    async fn get(&self) -> Result<TonicChannel, Error> {
        match self {
            Self::Endpoint(endpoint) => Ok(Endpoint::from_str(endpoint)?.connect().await?),
            Self::Tonic(cell) => match cell.get() {
                Some(channel) => Ok(channel.clone()),
                None => Err(Error::TransportChannelNotInitialized),
            },
        }
    }
}

#[derive(Clone)]
pub struct ClientConfig {
    region: String,
    host: String,
    https: bool,
    headers: HashMap<&'static str, String>,
}

impl ClientConfig {
    pub fn new(api_key: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            host: "api.topk.io".to_string(),
            https: true,
            headers: HashMap::from([("authorization", format!("Bearer {}", api_key.into()))]),
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

    pub fn with_headers(mut self, headers: HashMap<&'static str, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn headers(&self) -> HashMap<&'static str, String> {
        self.headers.clone()
    }

    pub fn endpoint(&self) -> String {
        let protocol = if self.https { "https" } else { "http" };

        format!("{}://{}.api.{}", protocol, self.region, self.host)
    }
}

#[derive(Clone)]
pub struct Client {
    config: ClientConfig,
    channel: Channel,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            channel: Channel::from_endpoint(config.endpoint().clone()),
            config,
        }
    }

    pub fn from_channel(channel: TonicChannel, config: ClientConfig) -> Self {
        Self {
            config,
            channel: Channel::from_tonic(channel),
        }
    }

    pub fn collections(&self) -> CollectionsClient {
        CollectionsClient::new(self.config.clone(), self.channel.clone())
    }

    pub fn collection(&self, name: impl Into<String>) -> CollectionClient {
        CollectionClient::new(self.config.clone(), self.channel.clone(), name.into())
    }
}

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

    pub async fn query(&self, query: Query) -> Result<Vec<Document>, Error> {
        self.query_at_lsn(query, None).await
    }

    pub async fn query_at_lsn(
        &self,
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
                .query_client()
                .await?
                .query(QueryRequest {
                    collection: self.collection.clone(),
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

    pub async fn upsert(&self, docs: Vec<Document>) -> Result<u64, Error> {
        let mut client = self.doc_client().await?;

        let response = client
            .upsert_documents(UpsertDocumentsRequest { docs })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::InvalidArgument => match ValidationErrorBag::try_from(e.clone()) {
                    Ok(errors) => Error::DocumentValidationError(errors),
                    Err(_) => Error::Unexpected(e),
                },
                tonic::Code::ResourceExhausted => Error::CapacityExceeded,
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().lsn)
    }

    pub async fn delete(&self, ids: Vec<String>) -> Result<u64, Error> {
        let mut client = self.doc_client().await?;

        let response = client
            .delete_documents(DeleteDocumentsRequest { ids })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::ResourceExhausted => Error::CapacityExceeded,
                tonic::Code::InvalidArgument => match ValidationErrorBag::try_from(e.clone()) {
                    Ok(errors) => Error::DocumentValidationError(errors),
                    Err(_) => Error::Unexpected(e),
                },
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().lsn)
    }

    async fn doc_client(&self) -> Result<DocumentClientWithHeaders, Error> {
        Ok(DocumentClient::with_headers(
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

pub struct CollectionsClient {
    config: ClientConfig,
    channel: Channel,
}

impl CollectionsClient {
    pub fn new(config: ClientConfig, channel: Channel) -> Self {
        Self { config, channel }
    }

    pub async fn list(&self) -> Result<Vec<Collection>, Error> {
        let response = self
            .client()
            .await?
            .list_collections(ListCollectionsRequest {})
            .await
            .map_err(|e| match e.code() {
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().collections)
    }

    pub async fn get(&self, name: impl Into<String>) -> Result<Collection, Error> {
        let response = self
            .client()
            .await?
            .get_collection(GetCollectionRequest { name: name.into() })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::NotFound => Error::CollectionNotFound,
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().collection.expect("invalid proto"))
    }

    pub async fn create(
        &self,
        name: impl Into<String>,
        schema: CollectionSchema,
    ) -> Result<Collection, Error> {
        let response = self
            .client()
            .await?
            .create_collection(CreateCollectionRequest {
                name: name.into(),
                schema: schema.into_fields(),
            })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::AlreadyExists => Error::CollectionAlreadyExists,
                tonic::Code::InvalidArgument => match ValidationErrorBag::try_from(e.clone()) {
                    Ok(errors) => Error::SchemaValidationError(errors),
                    Err(_) => Error::Unexpected(e),
                },
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().collection.expect("invalid proto"))
    }

    pub async fn delete(&self, name: impl Into<String>) -> Result<(), Error> {
        self.client()
            .await?
            .delete_collection(DeleteCollectionRequest { name: name.into() })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::NotFound => Error::CollectionNotFound,
                _ => Error::Unexpected(e),
            })?;

        Ok(())
    }

    //

    async fn client(&self) -> Result<CollectionClientWithHeaders, Error> {
        Ok(ProtoCollectionClient::with_headers(
            self.channel.get().await?,
            self.config.headers(),
        ))
    }
}
