use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use prost::Message;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::service::Routes;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Request, Response, Status};
use tower::{service_fn, ServiceExt};

use topk::config::Config;
use topk::host::Host;
use topk_rs::proto::v1::data::write_service_client::WriteServiceClient;
use topk_rs::proto::v1::data::write_service_server::{WriteService, WriteServiceServer};
use topk_rs::proto::v1::data::{
    DeleteDocumentsRequest, DeleteDocumentsResponse, UpdateDocumentsRequest,
    UpdateDocumentsResponse, UpsertDocumentsRequest, UpsertDocumentsResponse,
};

use super::Ctx;

pub struct Gate {
    pub config: Config,
    pub peak: Arc<AtomicUsize>,
    requests: mpsc::UnboundedReceiver<Pending>,
    server: JoinHandle<()>,
}

impl Gate {
    pub async fn start(ctx: &Ctx) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let data = ctx.client().config().endpoint().unwrap().connect_lazy();
        let scheme = if ctx.config.https() { "https" } else { "http" };
        let management = Endpoint::from_shared(format!("{scheme}://api.{}", ctx.config.host()))
            .unwrap()
            .connect_lazy();
        let (requests, received) = mpsc::unbounded_channel();
        let peak = Arc::new(AtomicUsize::new(0));
        let service = Writes {
            upstream: data.clone(),
            requests,
            active: Arc::new(AtomicUsize::new(0)),
            peak: peak.clone(),
        };
        let router = Routes::new(WriteServiceServer::new(service))
            .into_axum_router()
            .fallback_service(service_fn(move |request: http::Request<Body>| {
                let management = management.clone();
                let data = data.clone();
                async move {
                    let global = request
                        .uri()
                        .authority()
                        .is_some_and(|host| host.as_str().starts_with("api."));
                    let channel = if global { management } else { data };
                    let response = match channel.oneshot(request.map(tonic::body::Body::new)).await
                    {
                        Ok(response) => response.map(Body::new),
                        Err(error) => Status::unavailable(error.to_string())
                            .into_http::<tonic::body::Body>()
                            .map(Body::new),
                    };
                    Ok::<_, Infallible>(response)
                }
            }));
        let server = tokio::spawn(async move {
            Server::builder()
                .add_routes(router.into())
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap();
        });
        Self {
            config: Config::new(
                Host {
                    host: format!("ddb:{port}"),
                    https: false,
                },
                ctx.config.oauth().clone(),
                topk::config::Config::dir().unwrap(),
            )
            .with_project(Some(ctx.destination.project_id.clone())),
            peak,
            requests: received,
            server,
        }
    }

    pub async fn next(&mut self) -> Pending {
        timeout(Duration::from_secs(10), self.requests.recv())
            .await
            .expect("no upsert received in 10s")
            .expect("gate stopped")
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        self.server.abort();
    }
}

pub struct Pending {
    pub collection: String,
    pub partition: String,
    pub request: UpsertDocumentsRequest,
    pub forward: oneshot::Sender<Result<(), Status>>,
    pub committed: oneshot::Receiver<Result<(), tonic::Code>>,
    pub reply: oneshot::Sender<Result<(), Status>>,
}

impl Pending {
    pub async fn commit(self) -> oneshot::Sender<Result<(), Status>> {
        self.forward.send(Ok(())).unwrap();
        assert_eq!(
            timeout(Duration::from_secs(10), self.committed)
                .await
                .unwrap()
                .unwrap(),
            Ok(())
        );
        self.reply
    }

    pub fn bytes(&self) -> usize {
        self.request.docs.iter().map(Message::encoded_len).sum()
    }
}

struct Writes {
    upstream: Channel,
    requests: mpsc::UnboundedSender<Pending>,
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl WriteService for Writes {
    async fn upsert_documents(
        &self,
        request: Request<UpsertDocumentsRequest>,
    ) -> Result<Response<UpsertDocumentsResponse>, Status> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        let _active = Active(self.active.clone());
        let (forward, allowed) = oneshot::channel();
        let (committed, done) = oneshot::channel();
        let (reply, released) = oneshot::channel();
        let metadata = |name| {
            request
                .metadata()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string()
        };
        self.requests
            .send(Pending {
                collection: metadata("x-topk-collection"),
                partition: metadata("x-topk-partition"),
                request: request.get_ref().clone(),
                forward,
                committed: done,
                reply,
            })
            .map_err(|_| Status::cancelled("test stopped"))?;
        allowed
            .await
            .map_err(|_| Status::cancelled("forward cancelled"))??;
        let result = WriteServiceClient::new(self.upstream.clone())
            .upsert_documents(request)
            .await;
        let _ = committed.send(result.as_ref().map(|_| ()).map_err(Status::code));
        released
            .await
            .map_err(|_| Status::cancelled("reply cancelled"))??;
        result
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

struct Active(Arc<AtomicUsize>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
