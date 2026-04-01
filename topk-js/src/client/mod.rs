use std::{sync::Arc, time::Duration};

use collection::CollectionClient;
use collections::CollectionsClient;
use dataset::DatasetClient;
use datasets::DatasetsClient;
use futures_util::TryStreamExt;
use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::data::ask::{AskResult, ContextSearchResult, Mode, Source};
use crate::error::TopkError;
use crate::expr::logical::LogicalExpression;

pub mod collection;
pub mod collections;
pub mod dataset;
pub mod datasets;

/// Configuration for the TopK client.
///
/// This struct contains all the necessary configuration options to connect to the TopK API.
/// The `api_key` and `region` are required, while other options have sensible defaults.
#[napi(object)]
pub struct ClientConfig {
    /// Your TopK API key for authentication
    pub api_key: String,
    /// The region where your data is stored. For available regions see: https://docs.topk.io/regions.
    pub region: String,
    /// Custom host URL (optional, defaults to the standard TopK endpoint)
    pub host: Option<String>,
    /// Whether to use HTTPS (optional, defaults to true)
    pub https: Option<bool>,
    /// Retry configuration for failed requests (optional)
    pub retry_config: Option<RetryConfig>,
}

/// The main TopK client for interacting with the TopK service.
///
/// This client provides access to collections and allows you to perform various operations
/// like creating collections, querying data, and managing documents.
#[napi]
pub struct Client {
    client: Arc<topk_rs::Client>,
}

#[napi]
impl Client {
    /// Creates a new TopK client with the provided configuration.
    #[napi(constructor)]
    pub fn new(config: ClientConfig) -> Self {
        let mut rs_config = topk_rs::ClientConfig::new(config.api_key, config.region);

        if let Some(host_value) = config.host {
            rs_config = rs_config.with_host(host_value);
        }

        if let Some(https_value) = config.https {
            rs_config = rs_config.with_https(https_value);
        }

        if let Some(retry_config) = config.retry_config {
            rs_config = rs_config.with_retry_config(retry_config.into());
        }

        Self {
            client: Arc::new(topk_rs::Client::new(rs_config)),
        }
    }

    /// Returns a client for managing collections.
    ///
    /// This method provides access to collection management operations like creating,
    /// listing, and deleting collections.
    #[napi]
    pub fn collections(&self) -> CollectionsClient {
        CollectionsClient::new(self.client.clone())
    }

    /// Returns a client for interacting with a specific collection.
    #[napi]
    pub fn collection(&self, name: String) -> CollectionClient {
        CollectionClient::new(self.client.clone(), name)
    }

    /// Returns a client for managing datasets.
    ///
    /// This method provides access to dataset management operations like creating,
    /// listing, and deleting datasets.
    #[napi]
    pub fn datasets(&self) -> DatasetsClient {
        DatasetsClient::new(self.client.clone())
    }

    /// Returns a client for interacting with a specific dataset.
    #[napi]
    pub fn dataset(&self, name: String) -> DatasetClient {
        DatasetClient::new(self.client.clone(), name)
    }

    /// Queries context across one or more datasets and returns an answer.
    ///
    /// Consumes the streaming response and returns the final result.
    #[napi(
        ts_args_type = "query: string, sources: Array<string | Source>, filter?: query.LogicalExpression, mode?: Mode, selectFields?: Array<string>",
        ts_return_type = "Promise<AskResult>"
    )]
    pub async fn ask(
        &self,
        query: String,
        sources: Vec<Source>,
        filter: Option<&LogicalExpression>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> Result<AskResult> {
        let filter = filter.map(|f| f.clone().into());
        let mode = mode.map(|m| m.into());

        let stream = self
            .client
            .ask(query, sources, filter, mode, select_fields)
            .await
            .map_err(TopkError::from)?
            .into_inner();

        let result = stream
            .map_err(|e| napi::Error::from_reason(format!("stream error: {e}")))
            .try_fold(None, |_, result| async move { Ok(Some(result)) })
            .await?
            .ok_or_else(|| napi::Error::from_reason("ask returned no results"))?;

        match result.message {
            Some(inner) => AskResult::try_from(inner),
            None => Err(napi::Error::from_reason(
                "invalid proto: AskResult has no message",
            )),
        }
    }

    /// Searches across one or more datasets and returns matching results.
    ///
    /// Consumes the streaming response and collects all results.
    #[napi(
        ts_args_type = "query: string, sources: Array<string | Source>, topK: number, filter?: query.LogicalExpression, selectFields?: Array<string>",
        ts_return_type = "Promise<Array<ContextSearchResult>>"
    )]
    pub async fn search(
        &self,
        query: String,
        sources: Vec<Source>,
        top_k: u32,
        filter: Option<&LogicalExpression>,
        select_fields: Option<Vec<String>>,
    ) -> Result<Vec<ContextSearchResult>> {
        let filter = filter.map(|f| f.clone().into());
        let select_fields = select_fields.unwrap_or_default();

        let stream = self
            .client
            .search(query, sources, top_k, filter, select_fields)
            .await
            .map_err(TopkError::from)?
            .into_inner();

        stream
            .map_err(|e| napi::Error::from_reason(format!("stream error: {e}")))
            .and_then(|msg| std::future::ready(ContextSearchResult::try_from(msg)))
            .try_collect::<Vec<_>>()
            .await
    }
}

/// Configuration for retry behavior when requests fail.
///
/// This struct allows you to customize how the client handles retries for failed requests.
/// All fields are optional and will use sensible defaults if not provided.
#[napi(object)]
pub struct RetryConfig {
    /// Maximum number of retries to attempt before giving up
    pub max_retries: Option<u32>,

    /// Total timeout for the entire retry chain in milliseconds
    pub timeout: Option<u32>,

    /// Backoff configuration for spacing out retry attempts
    pub backoff: Option<BackoffConfig>,
}

impl Into<topk_rs::retry::RetryConfig> for RetryConfig {
    fn into(self) -> topk_rs::retry::RetryConfig {
        topk_rs::retry::RetryConfig {
            max_retries: self
                .max_retries
                .unwrap_or(topk_rs::defaults::RETRY_MAX_RETRIES as u32)
                as usize,
            timeout: Duration::from_millis(
                self.timeout
                    .unwrap_or(topk_rs::defaults::RETRY_TIMEOUT as u32) as u64,
            ),
            backoff: self.backoff.map(|b| b.into()).unwrap_or_default(),
        }
    }
}

/// Configuration for exponential backoff between retry attempts.
///
/// This struct controls how the delay between retry attempts increases over time.
/// All fields are optional and will use sensible defaults if not provided.
#[napi(object)]
pub struct BackoffConfig {
    /// Base multiplier for exponential backoff (default: 2.0)
    pub base: Option<u32>,

    /// Initial delay before the first retry in milliseconds
    pub init_backoff: Option<u32>,

    /// Maximum delay between retries in milliseconds
    pub max_backoff: Option<u32>,
}

impl Into<topk_rs::retry::BackoffConfig> for BackoffConfig {
    fn into(self) -> topk_rs::retry::BackoffConfig {
        topk_rs::retry::BackoffConfig {
            base: self.base.unwrap_or(topk_rs::defaults::RETRY_BACKOFF_BASE),
            init_backoff: Duration::from_millis(
                self.init_backoff
                    .unwrap_or(topk_rs::defaults::RETRY_BACKOFF_INIT as u32) as u64,
            ),
            max_backoff: Duration::from_millis(
                self.max_backoff
                    .unwrap_or(topk_rs::defaults::RETRY_BACKOFF_MAX as u32) as u64,
            ),
        }
    }
}
