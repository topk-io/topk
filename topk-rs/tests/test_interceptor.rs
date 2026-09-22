use std::error::Error as StdError;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::metadata::errors::InvalidMetadataValue;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Code, GrpcMethod, Request, Response, Status};

use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::client::AsyncInterceptor;
use topk_rs::proto::v1::control::collection_service_server::{
    CollectionService, CollectionServiceServer,
};
use topk_rs::proto::v1::control::{
    CreateCollectionRequest, CreateCollectionResponse, DeleteCollectionRequest,
    DeleteCollectionResponse, GetCollectionRequest, GetCollectionResponse, ListCollectionsRequest,
    ListCollectionsResponse,
};
use topk_rs::proto::v1::data::query_service_server::{QueryService, QueryServiceServer};
use topk_rs::proto::v1::data::write_service_server::{WriteService, WriteServiceServer};
use topk_rs::proto::v1::data::{
    DeleteDocumentsRequest, DeleteDocumentsResponse, UpdateDocumentsRequest,
    UpdateDocumentsResponse, UpsertDocumentsRequest, UpsertDocumentsResponse,
};
use topk_rs::proto::v1::data::{
    DocumentData, GetRequest, GetResponse, Query, QueryRequest, QueryResponse,
};
use topk_rs::{doc, Client, ClientConfig, Error};

struct Interceptor(AtomicUsize);

#[tonic::async_trait]
impl AsyncInterceptor for Interceptor {
    async fn call(&self, mut request: Request<()>) -> anyhow::Result<Request<()>> {
        let n = self.0.fetch_add(1, Ordering::SeqCst);
        anyhow::ensure!(n < 4, "login expired");
        assert_eq!(request.metadata().get("x-custom").unwrap(), "preserved");
        assert_eq!(
            request.metadata().get("authorization").unwrap(),
            "Bearer must-not-win"
        );
        assert!(!request
            .extensions()
            .get::<GrpcMethod>()
            .unwrap()
            .method()
            .is_empty());
        request
            .metadata_mut()
            .insert("x-intercepted", "yes".parse()?);
        assert!(request.metadata().contains_key("x-topk-sdk-version"));
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer token-{n}").parse()?);
        Ok(request)
    }
}

struct Service(mpsc::UnboundedSender<String>);

#[tonic::async_trait]
impl CollectionService for Service {
    async fn list_collections(
        &self,
        request: Request<ListCollectionsRequest>,
    ) -> Result<Response<ListCollectionsResponse>, Status> {
        let token = request
            .metadata()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(request.metadata().contains_key("x-topk-sdk-version"));
        assert_eq!(request.metadata().get("x-custom").unwrap(), "preserved");
        if token.starts_with("Bearer token-") {
            assert_eq!(request.metadata().get("x-intercepted").unwrap(), "yes");
        }
        self.0.send(token.into()).unwrap();
        if token == "Bearer token-0" {
            return Err(Status::unavailable("retry"));
        }
        Ok(Response::new(ListCollectionsResponse {
            collections: vec![],
        }))
    }
    async fn get_collection(
        &self,
        _: Request<GetCollectionRequest>,
    ) -> Result<Response<GetCollectionResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn create_collection(
        &self,
        _: Request<CreateCollectionRequest>,
    ) -> Result<Response<CreateCollectionResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn delete_collection(
        &self,
        _: Request<DeleteCollectionRequest>,
    ) -> Result<Response<DeleteCollectionResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
}

#[tonic::async_trait]
impl WriteService for Service {
    async fn upsert_documents(
        &self,
        request: Request<UpsertDocumentsRequest>,
    ) -> Result<Response<UpsertDocumentsResponse>, Status> {
        assert_eq!(
            request.get_ref().docs,
            vec![doc!("_id" => "book", "title" => "Test")]
        );
        assert_eq!(request.metadata().get("x-intercepted").unwrap(), "yes");
        assert_eq!(
            request.metadata().get("x-topk-collection").unwrap(),
            "books"
        );
        assert_eq!(
            request.metadata().get("x-topk-partition").unwrap(),
            "tenant"
        );
        assert!(request.metadata().contains_key("x-topk-sdk-version"));
        self.0
            .send(
                request
                    .metadata()
                    .get("authorization")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into(),
            )
            .unwrap();
        Ok(Response::new(UpsertDocumentsResponse { lsn: "1".into() }))
    }
    async fn update_documents(
        &self,
        _: Request<UpdateDocumentsRequest>,
    ) -> Result<Response<UpdateDocumentsResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn delete_documents(
        &self,
        _: Request<DeleteDocumentsRequest>,
    ) -> Result<Response<DeleteDocumentsResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
}

#[tonic::async_trait]
impl QueryService for Service {
    type QueryStreamStream = tokio_stream::Iter<std::vec::IntoIter<Result<DocumentData, Status>>>;
    type GetStreamStream = Self::QueryStreamStream;

    async fn query_stream(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::QueryStreamStream>, Status> {
        self.0
            .send(
                request
                    .metadata()
                    .get("authorization")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into(),
            )
            .unwrap();
        Ok(Response::new(tokio_stream::iter(vec![])))
    }
    async fn get_stream(
        &self,
        _: Request<GetRequest>,
    ) -> Result<Response<Self::GetStreamStream>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn query(&self, _: Request<QueryRequest>) -> Result<Response<QueryResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn get(&self, _: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
}

struct Fixture {
    channel: Channel,
    requests: mpsc::UnboundedReceiver<String>,
    _shutdown: oneshot::Sender<()>,
}

impl Fixture {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint =
            Endpoint::from_shared(format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let (tx, requests) = mpsc::unbounded_channel();
        let (shutdown, stopped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            Server::builder()
                .add_service(CollectionServiceServer::new(Service(tx.clone())))
                .add_service(WriteServiceServer::new(Service(tx.clone())))
                .add_service(QueryServiceServer::new(Service(tx)))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        Self {
            channel: endpoint.connect_lazy(),
            requests,
            _shutdown: shutdown,
        }
    }

    fn client(&self, interceptor: Arc<dyn AsyncInterceptor>) -> Client {
        Client::from_channel(
            ClientConfig::default()
                .with_region("test")
                .with_interceptor(interceptor)
                .with_headers([
                    ("authorization", "Bearer must-not-win"),
                    ("x-custom", "preserved"),
                ])
                .with_retry_config(RetryConfig {
                    max_retries: 3,
                    timeout: Duration::from_secs(5),
                    backoff: BackoffConfig {
                        init_backoff: Duration::from_millis(1),
                        ..BackoffConfig::default()
                    },
                }),
            self.channel.clone(),
        )
    }
}

#[tokio::test]
async fn interceptor_runs_on_each_attempt_and_errors_stop_before_dispatch() {
    let mut ctx = Fixture::new().await;
    let interceptor = Arc::new(Interceptor(AtomicUsize::new(0)));
    let client = ctx.client(interceptor.clone());
    client.collections().list().await.unwrap();
    assert_eq!(ctx.requests.recv().await.unwrap(), "Bearer token-0");
    assert_eq!(ctx.requests.recv().await.unwrap(), "Bearer token-1");
    let collection = client.clone().collection("books").partition("tenant");
    collection
        .upsert(vec![doc!("_id" => "book", "title" => "Test")])
        .await
        .unwrap();
    assert_eq!(ctx.requests.recv().await.unwrap(), "Bearer token-2");
    collection
        .upsert(vec![doc!("_id" => "book", "title" => "Test")])
        .await
        .unwrap();
    assert_eq!(ctx.requests.recv().await.unwrap(), "Bearer token-3");
    let error = client.collections().list().await.unwrap_err();
    assert!(matches!(error, Error::Interceptor(_)));
    assert!(error.to_string().contains("login expired"));
    assert_eq!(interceptor.0.load(Ordering::SeqCst), 5);
    assert!(ctx.requests.try_recv().is_err());
    let api_key = Client::from_channel(
        ClientConfig::new("api-key", "test").with_headers([("x-custom", "preserved")]),
        ctx.channel.clone(),
    );
    api_key.collections().list().await.unwrap();
    assert_eq!(ctx.requests.recv().await.unwrap(), "Bearer api-key");
}

struct FailingInterceptor {
    calls: AtomicUsize,
    response: Result<&'static str, Code>,
    context: bool,
}

#[tonic::async_trait]
impl AsyncInterceptor for FailingInterceptor {
    async fn call(&self, mut request: Request<()>) -> anyhow::Result<Request<()>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.response {
            Ok(token) => {
                request
                    .metadata_mut()
                    .insert("authorization", format!("Bearer {token}").parse()?);
                Ok(request)
            }
            Err(code) => {
                let error = anyhow::Error::new(Status::new(code, "interceptor failed"));
                Err(if self.context {
                    error.context("resolving credentials")
                } else {
                    error
                })
            }
        }
    }
}

#[tokio::test]
async fn interceptor_statuses_keep_their_sources_and_never_retry() {
    let mut ctx = Fixture::new().await;
    for (code, context) in [
        (Code::Unavailable, false),
        (Code::Unavailable, true),
        (Code::PermissionDenied, true),
        (Code::Unauthenticated, false),
    ] {
        let interceptor = Arc::new(FailingInterceptor {
            calls: AtomicUsize::new(0),
            response: Err(code),
            context,
        });
        let error = ctx
            .client(interceptor.clone())
            .collections()
            .list()
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Interceptor(_)), "{error:?}");
        assert!(!error.is_retryable());
        let status = std::iter::successors(error.source(), |e| (*e).source())
            .find_map(|e| e.downcast_ref::<Status>())
            .expect("original status remains in the source chain");
        assert_eq!(status.code(), code);
        assert_eq!(interceptor.calls.load(Ordering::SeqCst), 1);
        assert!(ctx.requests.try_recv().is_err());
    }
}

#[tokio::test]
async fn malformed_interceptor_token_never_reaches_server() {
    let mut ctx = Fixture::new().await;
    let interceptor = Arc::new(FailingInterceptor {
        calls: AtomicUsize::new(0),
        response: Ok("invalid\nheader"),
        context: false,
    });
    let error = ctx
        .client(interceptor.clone())
        .collections()
        .list()
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Interceptor(_)));
    assert!(!error.is_retryable());
    assert!(error.source().unwrap().is::<InvalidMetadataValue>());
    assert_eq!(interceptor.calls.load(Ordering::SeqCst), 1);
    assert!(ctx.requests.try_recv().is_err());
}

struct PendingInterceptor(AtomicBool);
struct Cancellation<'a>(&'a AtomicBool);

impl Drop for Cancellation<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[tonic::async_trait]
impl AsyncInterceptor for PendingInterceptor {
    async fn call(&self, _request: Request<()>) -> anyhow::Result<Request<()>> {
        let _guard = Cancellation(&self.0);
        std::future::pending().await
    }
}

#[tokio::test]
async fn retry_deadline_cancels_pending_interceptor() {
    let mut ctx = Fixture::new().await;
    let interceptor = Arc::new(PendingInterceptor(AtomicBool::new(false)));
    let client = Client::from_channel(
        ClientConfig::default()
            .with_region("test")
            .with_interceptor(interceptor.clone())
            .with_retry_config(RetryConfig {
                timeout: Duration::from_millis(50),
                ..RetryConfig::default()
            }),
        ctx.channel.clone(),
    );
    let error = client.collections().list().await.unwrap_err();
    assert!(matches!(error, Error::RetryTimeout));
    assert!(interceptor.0.load(Ordering::SeqCst));
    assert!(ctx.requests.try_recv().is_err());
}

#[tokio::test]
async fn streams_authenticate_once_at_initialization() {
    let mut ctx = Fixture::new().await;
    let interceptor = Arc::new(Interceptor(AtomicUsize::new(0)));
    let client = ctx.client(interceptor.clone());
    let mut stream = client
        .collection("books")
        .query_stream(Query::new(vec![]), None, None)
        .await
        .unwrap();
    assert_eq!(ctx.requests.recv().await.unwrap(), "Bearer token-0");
    assert!(stream.next().await.is_none());
    assert_eq!(interceptor.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn channel_failures_retain_retry_behavior() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint =
        Endpoint::from_shared(format!("http://{}", listener.local_addr().unwrap())).unwrap();
    drop(listener);
    let interceptor = Arc::new(FailingInterceptor {
        calls: AtomicUsize::new(0),
        response: Ok("token"),
        context: false,
    });
    let client = Client::from_channel(
        ClientConfig::default()
            .with_region("test")
            .with_interceptor(interceptor.clone())
            .with_retry_config(RetryConfig {
                max_retries: 2,
                timeout: Duration::from_secs(5),
                backoff: BackoffConfig {
                    init_backoff: Duration::from_millis(1),
                    ..BackoffConfig::default()
                },
            }),
        endpoint.connect_lazy(),
    );
    let error = client.collections().list().await.unwrap_err();
    assert!(error.is_retryable(), "{error:?}");
    assert_eq!(interceptor.calls.load(Ordering::SeqCst), 2);
}
