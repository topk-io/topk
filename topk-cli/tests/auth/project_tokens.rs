#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::timeout;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Endpoint, Server};
use tonic::{Request, Response, Status};

use topk::auth::{Auth, Config};
use topk::management::proto::data_plane_service_server::{
    DataPlaneService, DataPlaneServiceServer,
};
use topk::management::proto::region_service_server::{RegionService, RegionServiceServer};
use topk::management::proto::{
    ListRegionsRequest, ListRegionsResponse, MintAccessTokenRequest, MintAccessTokenResponse,
};
use topk::management::{Client as ManagementClient, ProjectTokenInterceptor, ProjectTokens};
use topk_rs::proto::v1::data::write_service_server::{WriteService, WriteServiceServer};
use topk_rs::proto::v1::data::{
    DeleteDocumentsRequest, DeleteDocumentsResponse, UpdateDocumentsRequest,
    UpdateDocumentsResponse, UpsertDocumentsRequest, UpsertDocumentsResponse,
};
use topk_rs::{doc, Client, ClientConfig, Error};

use super::common::{response, seed, tenant_dir, Server as OAuthServer};

type Reply = Result<MintAccessTokenResponse, Status>;

fn project_tokens(endpoint: Endpoint, config: &Config, config_dir: &Path) -> Arc<ProjectTokens> {
    let auth = Auth::new(config, config_dir.to_owned()).unwrap();
    let mint = Auth::new(config, config_dir.to_owned()).unwrap();
    Arc::new(ProjectTokens::new(
        ManagementClient::new(endpoint, mint),
        &auth,
    ))
}

struct Service {
    requests: mpsc::UnboundedSender<(String, String)>,
    replies: Mutex<mpsc::UnboundedReceiver<oneshot::Receiver<Reply>>>,
}

#[tonic::async_trait]
impl DataPlaneService for Service {
    async fn mint_access_token(
        &self,
        request: Request<MintAccessTokenRequest>,
    ) -> Result<Response<MintAccessTokenResponse>, Status> {
        let bearer = request
            .metadata()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        self.requests
            .send((request.into_inner().project_id, bearer))
            .unwrap();
        let reply = self
            .replies
            .lock()
            .await
            .recv()
            .await
            .ok_or_else(|| Status::unavailable("test server stopped"))?;
        reply
            .await
            .unwrap_or_else(|_| Err(Status::unavailable("test reply dropped")))
            .map(Response::new)
    }
}

#[tonic::async_trait]
impl RegionService for Service {
    async fn list_regions(
        &self,
        request: Request<ListRegionsRequest>,
    ) -> Result<Response<ListRegionsResponse>, Status> {
        assert_eq!(
            request.metadata().get("authorization").unwrap(),
            "Bearer access"
        );
        Ok(Response::new(ListRegionsResponse::default()))
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
            vec![doc!("_id" => "one", "title" => "Book")]
        );
        self.requests
            .send((
                request
                    .metadata()
                    .get("x-topk-collection")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into(),
                request
                    .metadata()
                    .get("authorization")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into(),
            ))
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

struct Fixture {
    oauth: OAuthServer,
    dir: TempDir,
    endpoint: Endpoint,
    requests: mpsc::UnboundedReceiver<(String, String)>,
    replies: mpsc::UnboundedSender<oneshot::Receiver<Reply>>,
    _shutdown: oneshot::Sender<()>,
}

impl Fixture {
    async fn new() -> Self {
        let oauth = OAuthServer::new().await;
        let dir = TempDir::new().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint =
            Endpoint::from_shared(format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let (requests_tx, requests) = mpsc::unbounded_channel();
        let (replies, replies_rx) = mpsc::unbounded_channel();
        let (shutdown, stopped) = oneshot::channel();
        tokio::spawn(async move {
            let service = Arc::new(Service {
                requests: requests_tx,
                replies: Mutex::new(replies_rx),
            });
            Server::builder()
                .add_service(DataPlaneServiceServer::from_arc(service.clone()))
                .add_service(RegionServiceServer::from_arc(service.clone()))
                .add_service(WriteServiceServer::from_arc(service))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        Self {
            oauth,
            dir,
            endpoint,
            requests,
            replies,
            _shutdown: shutdown,
        }
    }

    async fn login(&mut self, expires_in: u64) {
        self.oauth
            .reply(200, response(expires_in, Some("refresh")))
            .await;
        seed(&self.oauth.auth(self.dir.path())).await;
        self.oauth.request().await;
    }

    fn provider(&self) -> Arc<ProjectTokens> {
        project_tokens(self.endpoint.clone(), &self.oauth.config(), self.dir.path())
    }

    fn pending_reply(&self) -> oneshot::Sender<Reply> {
        let (sender, receiver) = oneshot::channel();
        self.replies.send(receiver).unwrap();
        sender
    }

    fn reply(&self, token: &str, seconds: i64) {
        self.pending_reply()
            .send(Ok(MintAccessTokenResponse {
                token: token.into(),
                expires_at: Utc::now().timestamp() + seconds,
            }))
            .unwrap();
    }

    fn credentials(&self) -> toml::Table {
        toml::from_str(
            &std::fs::read_to_string(
                tenant_dir(self.dir.path(), &self.oauth.url).join("credentials.toml"),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn token_path(&self, project: &str) -> PathBuf {
        self.projects_dir()
            .join("tokens")
            .join(format!("{project}.toml"))
    }

    fn lock_path(&self, project: &str) -> PathBuf {
        self.projects_dir()
            .join("locks")
            .join(format!("{project}.lock"))
    }

    fn projects_dir(&self) -> PathBuf {
        tenant_dir(self.dir.path(), &self.oauth.url).join("projects")
    }

    async fn request(&mut self, project: &str) {
        assert_eq!(
            timeout(Duration::from_secs(5), self.requests.recv())
                .await
                .unwrap()
                .unwrap(),
            (project.into(), "Bearer access".into())
        );
    }
}

#[tokio::test]
async fn cache_mints_once_and_renews_early() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    let credentials_path = tenant_dir(ctx.dir.path(), &ctx.oauth.url).join("credentials.toml");
    let credentials = std::fs::read(&credentials_path).unwrap();
    ctx.reply("one", 3600);
    let jobs: Vec<_> = (0..12)
        .map(|_| {
            let provider = ctx.provider();
            tokio::spawn(async move { provider.token("p1").await.unwrap().token })
        })
        .collect();
    for job in jobs {
        assert_eq!(job.await.unwrap(), "one");
    }
    ctx.request("p1").await;
    ctx.reply("early", 59);
    assert_eq!(ctx.provider().token("p2").await.unwrap().token, "early");
    ctx.request("p2").await;
    ctx.reply("renewed", 3600);
    assert_eq!(ctx.provider().token("p2").await.unwrap().token, "renewed");
    ctx.request("p2").await;
    assert_eq!(ctx.provider().token("p1").await.unwrap().token, "one");
    assert_eq!(ctx.provider().token("p2").await.unwrap().token, "renewed");
    assert!(ctx.requests.try_recv().is_err());
    assert!(ctx.oauth.requests.try_recv().is_err());
    assert_eq!(std::fs::read(&credentials_path).unwrap(), credentials);
    #[cfg(unix)]
    {
        assert_eq!(
            std::fs::metadata(ctx.token_path("p2"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

// Child process exercises the public provider using the parent's logged-in session.
#[tokio::test]
async fn mint_child() {
    let Ok(dir) = std::env::var("TOPK_TEST_TOKEN_DIR") else {
        return;
    };
    let config = Config {
        issuer: std::env::var("TOPK_TEST_TOKEN_ISSUER")
            .unwrap()
            .parse()
            .unwrap(),
        client_id: "test-client".into(),
        audience: "https://api.test".into(),
    };
    let provider = project_tokens(
        Endpoint::from_shared(std::env::var("TOPK_TEST_TOKEN_ENDPOINT").unwrap()).unwrap(),
        &config,
        Path::new(&dir),
    );
    assert_eq!(provider.token("p1").await.unwrap().token, "shared");
}

#[tokio::test]
async fn processes_share_one_mint() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    ctx.reply("shared", 3600);
    let mut children = Vec::new();
    for _ in 0..3 {
        children.push(
            tokio::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "project_tokens::mint_child"])
                .env("TOPK_TEST_TOKEN_DIR", ctx.dir.path())
                .env("TOPK_TEST_TOKEN_ISSUER", ctx.oauth.url.as_str())
                .env("TOPK_TEST_TOKEN_ENDPOINT", ctx.endpoint.uri().to_string())
                .kill_on_drop(true)
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(timeout(Duration::from_secs(10), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success());
    }
    ctx.request("p1").await;
    assert!(ctx.requests.try_recv().is_err());
}

#[tokio::test]
async fn mint_does_not_hold_the_session_lock() {
    let mut ctx = Fixture::new().await;
    ctx.login(0).await;
    ctx.oauth
        .reply(200, response(0, Some("first-rotation")))
        .await;
    let reply = ctx.pending_reply();
    let mut management =
        ManagementClient::new(ctx.endpoint.clone(), ctx.oauth.auth(ctx.dir.path()));
    let provider = ctx.provider();
    let mint = tokio::spawn(async move { provider.token("p1").await });
    ctx.oauth.request().await;
    ctx.request("p1").await;
    // Both refresh the account while the first mint is still pending.
    ctx.oauth
        .reply(200, response(3600, Some("second-rotation")))
        .await;
    timeout(
        Duration::from_secs(2),
        management.regions.list_regions(ListRegionsRequest {}),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(ctx
        .oauth
        .request()
        .await
        .contains("refresh_token=first-rotation"));
    ctx.reply("two", 3600);
    assert_eq!(
        timeout(Duration::from_secs(2), ctx.provider().token("p2"))
            .await
            .unwrap()
            .unwrap()
            .token,
        "two"
    );
    ctx.request("p2").await;
    reply
        .send(Ok(MintAccessTokenResponse {
            token: "one".into(),
            expires_at: Utc::now().timestamp() + 3600,
        }))
        .unwrap();
    assert_eq!(mint.await.unwrap().unwrap().token, "one");
    assert_eq!(
        ctx.credentials()["refresh_token"].as_str(),
        Some("second-rotation")
    );
    assert!(ctx.requests.try_recv().is_err());
}

#[tokio::test]
async fn cached_token_needs_no_account_but_renewal_uses_current_login() {
    let mut ctx = Fixture::new().await;
    ctx.login(0).await;
    ctx.oauth.reply(200, response(0, Some("rotated"))).await;
    ctx.reply("cached", 3600);
    let provider = ctx.provider();
    assert_eq!(provider.token("p1").await.unwrap().token, "cached");
    ctx.oauth.request().await;
    ctx.request("p1").await;
    // A rejected refresh removes the account session but keeps project tokens.
    ctx.oauth
        .reply(400, serde_json::json!({"error": "invalid_grant"}))
        .await;
    assert!(ctx.oauth.auth(ctx.dir.path()).access_token().await.is_err());
    ctx.oauth.request().await;
    assert_eq!(provider.token("p1").await.unwrap().token, "cached");
    let path = ctx.token_path("p1");
    let mut token: toml::Table = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    token["expires_at"] = (Utc::now().timestamp() - 1).into();
    std::fs::write(path, toml::to_string(&token).unwrap()).unwrap();
    let error = provider.token("p1").await.err().unwrap();
    assert!(format!("{error:#}").contains("not logged in"));
    let mut account = response(3600, Some("new-refresh"));
    account["access_token"] = "new-account".into();
    ctx.oauth.reply(200, account).await;
    seed(&ctx.oauth.auth(ctx.dir.path())).await;
    ctx.oauth.request().await;
    ctx.reply("renewed", 3600);
    assert_eq!(provider.token("p1").await.unwrap().token, "renewed");
    assert_eq!(
        ctx.requests.recv().await.unwrap(),
        ("p1".into(), "Bearer new-account".into())
    );
}

#[tokio::test]
async fn rejected_account_session_suggests_login() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    ctx.pending_reply()
        .send(Err(Status::unauthenticated("invalid token")))
        .unwrap();
    let error = ctx.provider().token("p1").await.err().unwrap();
    assert!(error.to_string().contains("Run `topk login`"));
    assert_eq!(
        error.downcast_ref::<Status>().unwrap().code(),
        tonic::Code::Unauthenticated
    );
    ctx.request("p1").await;
}

#[tokio::test]
async fn cancellation_releases_project_lock() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    let _reply = ctx.pending_reply();
    let provider = ctx.provider();
    let mint = tokio::spawn(async move { provider.token("p1").await });
    ctx.request("p1").await;
    mint.abort();
    assert!(mint.await.err().unwrap().is_cancelled());
    ctx.reply("retry", 3600);
    assert_eq!(
        timeout(Duration::from_secs(2), ctx.provider().token("p1"))
            .await
            .unwrap()
            .unwrap()
            .token,
        "retry"
    );
}

#[tokio::test]
async fn corrupt_project_token_is_repaired_once() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    let path = ctx.token_path("p1");
    for malformed in ["invalid TOML", "token = 'missing expiry'"] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, malformed).unwrap();
        ctx.reply("repaired", 3600);
        let first = ctx.provider();
        let second = ctx.provider();
        let (first, second) = tokio::join!(first.token("p1"), second.token("p1"));
        assert_eq!(first.unwrap().token, "repaired");
        assert_eq!(second.unwrap().token, "repaired");
        ctx.request("p1").await;
        assert!(ctx.requests.try_recv().is_err());
    }
}

#[tokio::test]
async fn shared_tokens_bind_each_sdk_client_to_its_project() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    let tokens = ctx.provider();
    let client = |project: &str| {
        Client::from_channel(
            ClientConfig::default()
                .with_region("test")
                .with_interceptor(Arc::new(ProjectTokenInterceptor::new(
                    tokens.clone(),
                    project.into(),
                ))),
            ctx.endpoint.connect_lazy(),
        )
    };
    let clients = [("p1", "one", client("p1")), ("p2", "two", client("p2"))];
    for mint in [true, false] {
        for (project, token, client) in &clients {
            if mint {
                ctx.reply(token, 3600);
            }
            client
                .collection("books")
                .upsert(vec![doc!("_id" => "one", "title" => "Book")])
                .await
                .unwrap();
            if mint {
                ctx.request(project).await;
            }
            assert_eq!(
                ctx.requests.recv().await.unwrap(),
                ("books".into(), format!("Bearer {token}"))
            );
        }
    }
    assert!(ctx.requests.try_recv().is_err());
    assert!(ctx.oauth.requests.try_recv().is_err());
}

#[tokio::test]
async fn failed_mint_stops_upsert_without_retrying() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    ctx.pending_reply()
        .send(Err(Status::unavailable("mint service unavailable")))
        .unwrap();
    let client = Client::from_channel(
        ClientConfig::default()
            .with_region("test")
            .with_interceptor(Arc::new(ProjectTokenInterceptor::new(
                ctx.provider(),
                "p1".into(),
            ))),
        ctx.endpoint.connect_lazy(),
    );
    let error = client
        .collection("books")
        .upsert(vec![doc!("_id" => "one", "title" => "Book")])
        .await
        .unwrap_err();
    let Error::Interceptor(source) = error else {
        panic!("expected interceptor error, got {error:?}");
    };
    assert_eq!(
        source.downcast_ref::<Status>().unwrap().code(),
        tonic::Code::Unavailable
    );
    ctx.request("p1").await;
    assert!(ctx.requests.try_recv().is_err());
}

#[tokio::test]
async fn clear_removes_project_tokens_and_preserves_lock_files() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    for project in ["p1", "p2"] {
        ctx.reply(project, 3600);
        ctx.provider().token(project).await.unwrap();
        ctx.request(project).await;
    }
    let lock_path = ctx.lock_path("p1");
    let lock = std::fs::File::open(&lock_path).unwrap();
    lock.lock().unwrap();
    for _ in 0..2 {
        ProjectTokens::clear(&ctx.oauth.auth(ctx.dir.path())).unwrap();
    }
    for project in ["p1", "p2"] {
        assert!(!ctx.token_path(project).exists());
    }
    let same_lock = std::fs::File::open(lock_path).unwrap();
    assert!(matches!(
        same_lock.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
}

#[tokio::test]
async fn project_ids_cannot_escape_the_cache() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    let error = ctx.provider().token("../p1").await.err().unwrap();
    assert!(error.to_string().contains("invalid project ID"));
    assert!(ctx.requests.try_recv().is_err());
}
