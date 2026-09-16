use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Endpoint, Server};
use tonic::{Code, Request, Response, Status};

use topk::management::proto::collection_service_server::{
    CollectionService, CollectionServiceServer,
};
use topk::management::proto::project_service_server::{ProjectService, ProjectServiceServer};
use topk::management::proto::region_service_server::{RegionService, RegionServiceServer};
use topk::management::proto::*;
use topk::management::ManagementClient;
use topk_rs::proto::v1::control::FieldSpec;

use super::common::{response, seed, Server as OAuthServer};

#[derive(Clone)]
struct Services {
    requests: mpsc::UnboundedSender<(String, String)>,
}

impl Services {
    fn record<T>(&self, method: &str, request: &Request<T>) {
        self.requests
            .send((
                method.into(),
                request
                    .metadata()
                    .get("authorization")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into(),
            ))
            .unwrap();
    }
}

#[tonic::async_trait]
impl ProjectService for Services {
    async fn list_projects(
        &self,
        request: Request<ListProjectsRequest>,
    ) -> Result<Response<ListProjectsResponse>, Status> {
        self.record("projects", &request);
        Ok(Response::new(ListProjectsResponse {
            projects: vec![Project {
                project_id: "project-1".into(),
                name: "Example".into(),
                ..Default::default()
            }],
        }))
    }

    async fn get_project(
        &self,
        _: Request<GetProjectRequest>,
    ) -> Result<Response<GetProjectResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }

    async fn create_project(
        &self,
        request: Request<CreateProjectRequest>,
    ) -> Result<Response<CreateProjectResponse>, Status> {
        self.record("create", &request);
        assert_eq!(request.into_inner().name, "new-project");
        Err(Status::unavailable("try later"))
    }

    async fn delete_project(
        &self,
        _: Request<DeleteProjectRequest>,
    ) -> Result<Response<DeleteProjectResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
}

#[tonic::async_trait]
impl CollectionService for Services {
    async fn list_collections(
        &self,
        request: Request<ListCollectionsRequest>,
    ) -> Result<Response<ListCollectionsResponse>, Status> {
        self.record("collections", &request);
        assert_eq!(request.into_inner().project_id, "project-1");
        Ok(Response::new(ListCollectionsResponse {
            collections: vec![Collection {
                name: "books".into(),
                project_id: "project-1".into(),
                schema: [("title".into(), FieldSpec::default())].into(),
                ..Default::default()
            }],
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
impl RegionService for Services {
    async fn list_regions(
        &self,
        request: Request<ListRegionsRequest>,
    ) -> Result<Response<ListRegionsResponse>, Status> {
        self.record("regions", &request);
        Ok(Response::new(ListRegionsResponse {
            regions: vec!["aws-us-east-1".into()],
        }))
    }
}

struct Fixture {
    oauth: OAuthServer,
    dir: TempDir,
    client: ManagementClient,
    requests: mpsc::UnboundedReceiver<(String, String)>,
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
        let services = Services {
            requests: requests_tx,
        };
        let (shutdown, stopped) = oneshot::channel();
        tokio::spawn(async move {
            Server::builder()
                .add_service(ProjectServiceServer::new(services.clone()))
                .add_service(CollectionServiceServer::new(services.clone()))
                .add_service(RegionServiceServer::new(services))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        let client = ManagementClient::new(endpoint, oauth.auth(dir.path()));
        Self {
            oauth,
            dir,
            client,
            requests,
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

    async fn request(&mut self, method: &str, token: &str) {
        assert_eq!(
            timeout(Duration::from_secs(5), self.requests.recv())
                .await
                .unwrap()
                .unwrap(),
            (method.into(), format!("Bearer {token}"))
        );
    }
}

#[tokio::test]
async fn services_send_bearer_tokens_and_preserve_responses_and_errors() {
    let mut ctx = Fixture::new().await;
    ctx.login(3600).await;
    let projects = ctx
        .client
        .projects
        .list_projects(ListProjectsRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(projects.projects[0].project_id, "project-1");
    ctx.request("projects", "access").await;
    let collections = ctx
        .client
        .collections
        .list_collections(ListCollectionsRequest {
            project_id: "project-1".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(collections.collections[0].name, "books");
    assert!(collections.collections[0].schema.contains_key("title"));
    ctx.request("collections", "access").await;
    let regions = ctx
        .client
        .regions
        .list_regions(ListRegionsRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(regions.regions, ["aws-us-east-1"]);
    ctx.request("regions", "access").await;
    let error = ctx
        .client
        .projects
        .create_project(CreateProjectRequest {
            name: "new-project".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unavailable);
    assert_eq!(error.message(), "try later");
    ctx.request("create", "access").await;
    assert!(ctx.requests.try_recv().is_err());
    assert!(ctx.oauth.requests.try_recv().is_err());
}

#[tokio::test]
async fn existing_service_client_refreshes_and_observes_logout() {
    let mut ctx = Fixture::new().await;
    let mut regions = ctx.client.regions.clone();
    ctx.login(3600).await;
    regions.list_regions(ListRegionsRequest {}).await.unwrap();
    ctx.request("regions", "access").await;
    // Replace the session through a real login to expire it without sleeping.
    ctx.login(0).await;
    ctx.oauth.reply(200, json!({"access_token": "renewed", "token_type": "Bearer", "expires_in": 3600, "refresh_token": "rotated"})).await;
    regions.list_regions(ListRegionsRequest {}).await.unwrap();
    assert!(ctx
        .oauth
        .request()
        .await
        .contains("grant_type=refresh_token"));
    ctx.request("regions", "renewed").await;
    regions.list_regions(ListRegionsRequest {}).await.unwrap();
    ctx.request("regions", "renewed").await;
    ctx.oauth.auth(ctx.dir.path()).logout().await.unwrap();
    let error = regions
        .list_regions(ListRegionsRequest {})
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unknown);
    assert!(error.message().contains("topk login"));
    assert!(ctx.requests.try_recv().is_err());
    assert!(ctx.oauth.requests.try_recv().is_err());
}

#[tokio::test]
async fn concurrent_service_calls_refresh_once() {
    let mut ctx = Fixture::new().await;
    ctx.login(0).await;
    ctx.oauth.reply(200, response(3600, Some("rotated"))).await;
    let mut jobs = Vec::new();
    for _ in 0..8 {
        let mut client = ctx.client.clone();
        jobs.push(tokio::spawn(async move {
            client
                .regions
                .list_regions(ListRegionsRequest {})
                .await
                .unwrap()
        }));
    }
    for job in jobs {
        job.await.unwrap();
        ctx.request("regions", "access").await;
    }
    assert!(ctx.oauth.request().await.contains("refresh_token=refresh"));
    assert!(ctx.oauth.requests.try_recv().is_err());
}

#[tokio::test]
async fn authentication_failures_never_reach_management_server() {
    let mut ctx = Fixture::new().await;
    let error = ctx
        .client
        .projects
        .list_projects(ListProjectsRequest {})
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unknown);
    assert!(error.message().contains("not logged in"));
    assert!(std::error::Error::source(&error).is_some());
    ctx.login(0).await;
    ctx.oauth.reply(503, json!({"error": "server_error"})).await;
    let error = ctx
        .client
        .projects
        .list_projects(ListProjectsRequest {})
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unknown);
    assert!(error.message().contains("refreshing the access token"));
    assert!(anyhow::Error::new(error)
        .chain()
        .any(|cause| cause.to_string().contains("server_error")));
    ctx.oauth.request().await;
    assert!(ctx.requests.try_recv().is_err());
    ctx.oauth
        .reply(400, json!({"error": "invalid_grant"}))
        .await;
    let error = ctx
        .client
        .projects
        .list_projects(ListProjectsRequest {})
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unknown);
    assert!(error
        .message()
        .contains("session expired. Run `topk login`."));
    assert!(std::error::Error::source(&error).is_some());
    ctx.oauth.request().await;
    assert!(ctx.requests.try_recv().is_err());
}
