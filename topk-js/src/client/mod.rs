use std::{sync::Arc, thread, time::Duration};

use collection::CollectionClient;
use collections::CollectionsClient;
use dataset::DatasetClient;
use datasets::DatasetsClient;
use futures_util::{StreamExt, TryStreamExt};
use napi::bindgen_prelude::*;
use napi::tokio::{
    self,
    sync::{mpsc, Mutex},
};
use napi_derive::napi;

use napi::bindgen_prelude::Either3;

use crate::data::ask::{
    convert_ask_result, convert_ask_result_to_answer, Answer, Mode, Reason, Search, SearchResult,
    Source,
};
use crate::error::TopkError;
use crate::expr::logical::LogicalExpression;

pub mod collection;
pub mod collections;
pub mod dataset;
pub mod datasets;

pub(crate) const STREAM_BUFFER_SIZE: usize = 16;
type AskStreamMessage = std::result::Result<Either3<Answer, Search, Reason>, String>;
type SearchStreamMessage = std::result::Result<SearchResult, String>;

pub(crate) fn spawn_stream_task<F>(task: F)
where
    F: FnOnce() -> std::result::Result<(), String> + Send + 'static,
{
    thread::spawn(move || {
        if let Err(error) = task() {
            eprintln!("topk-js stream task failed: {error}");
        }
    });
}

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

/// Client for interacting with the TopK API. For available regions see https://docs.topk.io/regions
#[napi]
pub struct Client {
    client: Arc<topk_rs::Client>,
}

/// Iterator for ask responses.
#[napi(async_iterator)]
pub struct AskStream {
    receiver: Arc<Mutex<mpsc::Receiver<AskStreamMessage>>>,
}

impl AskStream {
    fn new(receiver: mpsc::Receiver<AskStreamMessage>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}

#[napi]
impl AsyncGenerator for AskStream {
    type Yield = Either3<Answer, Search, Reason>;
    type Next = ();
    type Return = ();

    fn next(
        &mut self,
        _value: Option<Self::Next>,
    ) -> impl std::future::Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
        let receiver = self.receiver.clone();
        async move {
            let mut guard = receiver.lock().await;
            match guard.recv().await {
                Some(Ok(v)) => Ok(Some(v)),
                Some(Err(e)) => Err(napi::Error::from_reason(e)),
                None => Ok(None),
            }
        }
    }
}

/// Iterator for search responses.
#[napi(async_iterator)]
pub struct SearchStream {
    receiver: Arc<Mutex<mpsc::Receiver<SearchStreamMessage>>>,
}

impl SearchStream {
    fn new(receiver: mpsc::Receiver<SearchStreamMessage>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}

#[napi]
impl AsyncGenerator for SearchStream {
    type Yield = SearchResult;
    type Next = ();
    type Return = ();

    fn next(
        &mut self,
        _value: Option<Self::Next>,
    ) -> impl std::future::Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
        let receiver = self.receiver.clone();
        async move {
            let mut guard = receiver.lock().await;
            match guard.recv().await {
                Some(Ok(v)) => Ok(Some(v)),
                Some(Err(e)) => Err(napi::Error::from_reason(e)),
                None => Ok(None),
            }
        }
    }
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

    /// Get a client for managing datasets.
    #[napi]
    pub fn datasets(&self) -> DatasetsClient {
        DatasetsClient::new(self.client.clone())
    }

    /// Get a client for managing data operations on a specific dataset such as upserting files, managing metadata, and deleting files.
    #[napi]
    pub fn dataset(&self, name: String) -> DatasetClient {
        DatasetClient::new(self.client.clone(), name)
    }

    /// Ask a question and wait for the stream to complete, returning the last message.
    #[napi(
        ts_args_type = "query: string, datasets: Array<string | { dataset: string; filter?: query.LogicalExpression }>, filter?: query.LogicalExpression, mode?: Mode, selectFields?: Array<string>"
    )]
    pub async fn ask(
        &self,
        query: String,
        datasets: Vec<Source>,
        filter: Option<&LogicalExpression>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> Result<Answer> {
        let filter = filter.map(|f| f.clone().into());
        let mode = mode.map(|m| m.into());

        let stream = self
            .client
            .ask(query, datasets, filter, mode, select_fields)
            .await
            .map_err(TopkError::from)?
            .into_inner();

        let result = stream
            .map_err(|e| napi::Error::from_reason(format!("stream error: {e}")))
            .try_fold(None, |_, result| async move { Ok(Some(result)) })
            .await?
            .ok_or_else(|| napi::Error::from_reason("ask returned no results"))?;

        convert_ask_result_to_answer(result)
    }

    /// Ask a question and get streaming responses as an async iterator.
    #[napi(
        ts_args_type = "query: string, datasets: Array<string | { dataset: string; filter?: query.LogicalExpression }>, filter?: query.LogicalExpression, mode?: Mode, selectFields?: Array<string>"
    )]
    pub fn ask_stream(
        &self,
        query: String,
        datasets: Vec<Source>,
        filter: Option<&LogicalExpression>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> AskStream {
        let (tx, rx) = mpsc::channel::<AskStreamMessage>(STREAM_BUFFER_SIZE);
        let client = self.client.clone();
        let filter = filter.map(|f| f.clone().into());
        let mode = mode.map(|m| m.into());

        spawn_stream_task(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("failed to build tokio runtime: {e}"))?;

            runtime.block_on(async move {
                let mut stream = match client
                    .ask(query, datasets, filter, mode, select_fields)
                    .await
                {
                    Ok(response) => response.into_inner(),
                    Err(error) => {
                        let _ = tx.send(Err(format!("{error}"))).await;
                        return;
                    }
                };

                while let Some(result) = stream.next().await {
                    let message = match result {
                        Ok(message) => convert_ask_result(message).map_err(|e| e.to_string()),
                        Err(error) => Err(format!("stream error: {error}")),
                    };

                    if tx.send(message).await.is_err() {
                        break;
                    }
                }
            });

            Ok(())
        });

        AskStream::new(rx)
    }

    /// Search for documents and wait for the stream to complete, returning all results.
    #[napi(
        ts_args_type = "query: string, datasets: Array<string | { dataset: string; filter?: query.LogicalExpression }>, topK: number, filter?: query.LogicalExpression, selectFields?: Array<string>"
    )]
    pub async fn search(
        &self,
        query: String,
        datasets: Vec<Source>,
        top_k: u32,
        filter: Option<&LogicalExpression>,
        select_fields: Option<Vec<String>>,
    ) -> Result<Vec<SearchResult>> {
        let filter = filter.map(|f| f.clone().into());
        let select_fields = select_fields.unwrap_or_default();

        let stream = self
            .client
            .search(query, datasets, top_k, filter, select_fields)
            .await
            .map_err(TopkError::from)?
            .into_inner();

        stream
            .map_err(|e| napi::Error::from_reason(format!("stream error: {e}")))
            .and_then(|msg| std::future::ready(SearchResult::try_from(msg)))
            .try_collect::<Vec<_>>()
            .await
    }

    /// Search for documents and get streaming responses as an async iterator.
    #[napi(
        ts_args_type = "query: string, datasets: Array<string | { dataset: string; filter?: query.LogicalExpression }>, topK: number, filter?: query.LogicalExpression, selectFields?: Array<string>"
    )]
    pub fn search_stream(
        &self,
        query: String,
        datasets: Vec<Source>,
        top_k: u32,
        filter: Option<&LogicalExpression>,
        select_fields: Option<Vec<String>>,
    ) -> SearchStream {
        let (tx, rx) = mpsc::channel::<SearchStreamMessage>(STREAM_BUFFER_SIZE);
        let client = self.client.clone();
        let filter = filter.map(|f| f.clone().into());
        let select_fields = select_fields.unwrap_or_default();

        spawn_stream_task(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("failed to build tokio runtime: {e}"))?;

            runtime.block_on(async move {
                let mut stream = match client
                    .search(query, datasets, top_k, filter, select_fields)
                    .await
                {
                    Ok(response) => response.into_inner(),
                    Err(error) => {
                        let _ = tx.send(Err(format!("{error}"))).await;
                        return;
                    }
                };

                while let Some(result) = stream.next().await {
                    let message = match result {
                        Ok(message) => SearchResult::try_from(message).map_err(|e| e.to_string()),
                        Err(error) => Err(format!("stream error: {error}")),
                    };

                    if tx.send(message).await.is_err() {
                        break;
                    }
                }
            });

            Ok(())
        });

        SearchStream::new(rx)
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
