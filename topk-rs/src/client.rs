use crate::errors::ValidationErrorBag;
use crate::{Error, InternalErrorCode};
use std::str::FromStr;
use std::{collections::HashMap, time::Duration};
use tokio::sync::OnceCell;
use tonic::transport::{Channel as TonicChannel, Endpoint};
use topk_protos::utils::{
    CollectionClient as ProtoCollectionClient, CollectionClientWithHeaders,
    DocumentClientWithHeaders, QueryClientWithHeaders,
};
use topk_protos::v1::control::{FieldSpec, GetCollectionRequest};
use topk_protos::v1::data::{ConsistencyLevel, GetRequest};
use topk_protos::{
    utils::{DocumentClient, QueryClient},
    v1::{
        control::{
            Collection, CreateCollectionRequest, DeleteCollectionRequest, ListCollectionsRequest,
        },
        data::{DeleteDocumentsRequest, Document, Query, QueryRequest, UpsertDocumentsRequest},
    },
};

#[derive(Debug, Clone)]
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
            Self::Endpoint(endpoint) => Ok(Endpoint::from_str(endpoint)?
                .tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())?
                // Do not close idle connections so they can be reused
                .keep_alive_while_idle(true)
                // Set max header list size to 64KB
                .http2_max_header_list_size(1024 * 64)
                .connect()
                .await?),
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
            host: "topk.io".to_string(),
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
        id: impl Into<String>,
        fields: Vec<String>,
        lsn: Option<u64>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<Document, Error> {
        let mut tries = 0;
        let max_tries = 120;
        let retry_after = Duration::from_secs(1);

        let id = id.into();

        loop {
            tries += 1;

            let fields = fields.clone();

            let response = self
                .query_client()
                .await?
                .get(GetRequest {
                    id: id.clone(),
                    fields,
                    required_lsn: lsn,
                    consistency_level: consistency.map(|c| c.into()),
                })
                .await;

            match response {
                Ok(response) => match response.into_inner().doc {
                    Some(doc) => return Ok(doc),
                    None => return Err(Error::InvalidProto),
                },
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
                                tonic::Code::NotFound => Error::DocumentNotFound,
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

    pub async fn query(
        &self,
        query: Query,
        lsn: Option<u64>,
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
                    query: Some(query),
                    required_lsn: lsn,
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

    pub async fn upsert(&self, docs: Vec<Document>) -> Result<u64, Error> {
        let mut client = self.doc_client().await?;

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

    pub async fn delete(&self, ids: Vec<String>) -> Result<u64, Error> {
        let mut client = self.doc_client().await?;

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
        schema: impl Into<HashMap<String, FieldSpec>>,
    ) -> Result<Collection, Error> {
        let response = self
            .client()
            .await?
            .create_collection(CreateCollectionRequest {
                name: name.into(),
                schema: schema.into(),
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
