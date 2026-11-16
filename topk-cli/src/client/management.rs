use anyhow::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use tonic::transport::{ClientTlsConfig, Endpoint};
use tonic::Status;

use crate::auth::Auth;
use crate::client::transport::Transport;
use crate::client::AccessToken;
use crate::config::Config;
use crate::management::proto::collection_service_client::CollectionServiceClient;
use crate::management::proto::data_plane_service_client::DataPlaneServiceClient;
use crate::management::proto::project_service_client::ProjectServiceClient;
use crate::management::proto::region_service_client::RegionServiceClient;
use crate::management::proto::usage_service_client::UsageServiceClient;
use crate::management::proto::{
    Collection, CreateProjectRequest, DeleteProjectRequest, GetCollectionRequest,
    GetProjectRequest, GetUsageRequest, ListProjectsRequest, ListRegionsRequest,
    MintAccessTokenRequest, Project, UsageMetric,
};
use crate::ProjectId;

#[derive(Clone)]
pub struct ManagementClient {
    transport: Transport,
}

impl ManagementClient {
    pub fn new(config: Config) -> Result<Self> {
        let mut grpc = Endpoint::from_shared(config.api_url())?;
        if config.https() {
            grpc = grpc.tls_config(ClientTlsConfig::new().with_native_roots())?;
        }
        Ok(Self::connect(grpc, Auth::new(config)?))
    }

    pub fn connect(grpc: Endpoint, auth: Auth) -> Self {
        Self {
            transport: Transport::new(grpc.connect_lazy(), auth),
        }
    }

    pub fn projects(&self) -> ProjectsClient {
        ProjectsClient(ProjectServiceClient::new(self.transport.clone()))
    }

    pub fn collections(&self) -> CollectionsClient {
        CollectionsClient(CollectionServiceClient::new(self.transport.clone()))
    }

    pub fn regions(&self) -> RegionsClient {
        RegionsClient(RegionServiceClient::new(self.transport.clone()))
    }

    pub fn tokens(&self) -> TokensClient {
        TokensClient(DataPlaneServiceClient::new(self.transport.clone()))
    }

    pub fn usage(&self) -> UsageClient {
        UsageClient(UsageServiceClient::new(self.transport.clone()))
    }
}

pub struct ProjectsClient(ProjectServiceClient<Transport>);

impl ProjectsClient {
    pub async fn list(&self) -> Result<Vec<Project>, Status> {
        let response = self.0.clone().list_projects(ListProjectsRequest {}).await?;
        Ok(response.into_inner().projects)
    }

    pub async fn get(&self, project_id: &ProjectId) -> Result<Project, Status> {
        self.0
            .clone()
            .get_project(GetProjectRequest {
                project_id: project_id.to_string(),
            })
            .await?
            .into_inner()
            .project
            .ok_or_else(|| Status::internal("invalid proto"))
    }

    pub async fn create(&self, name: &str) -> Result<Project, Status> {
        self.0
            .clone()
            .create_project(CreateProjectRequest {
                name: name.to_string(),
            })
            .await?
            .into_inner()
            .project
            .ok_or_else(|| Status::internal("invalid proto"))
    }

    pub async fn delete(&self, project_id: &ProjectId) -> Result<(), Status> {
        self.0
            .clone()
            .delete_project(DeleteProjectRequest {
                project_id: project_id.to_string(),
            })
            .await?;
        Ok(())
    }
}

pub struct CollectionsClient(CollectionServiceClient<Transport>);

impl CollectionsClient {
    pub async fn get(&self, project_id: &ProjectId, name: &str) -> Result<Collection, Status> {
        self.0
            .clone()
            .get_collection(GetCollectionRequest {
                project_id: project_id.to_string(),
                name: name.to_string(),
            })
            .await?
            .into_inner()
            .collection
            .ok_or_else(|| Status::internal("invalid proto"))
    }
}

pub struct RegionsClient(RegionServiceClient<Transport>);

impl RegionsClient {
    pub async fn list(&self) -> Result<Vec<String>, Status> {
        let response = self.0.clone().list_regions(ListRegionsRequest {}).await?;
        Ok(response.into_inner().regions)
    }
}

pub struct TokensClient(DataPlaneServiceClient<Transport>);

impl TokensClient {
    pub async fn mint(&self, project_id: &ProjectId) -> Result<AccessToken, Status> {
        let response = self
            .0
            .clone()
            .mint_access_token(MintAccessTokenRequest {
                project_id: project_id.to_string(),
            })
            .await?
            .into_inner();
        Ok(AccessToken {
            token: response.token,
            expires_at: response
                .expires_at
                .try_into()
                .map_err(|_| Status::internal("invalid proto"))?,
        })
    }
}

pub struct UsageClient(UsageServiceClient<Transport>);

impl UsageClient {
    pub async fn get(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        project_id: Option<&ProjectId>,
        collection: Option<String>,
    ) -> Result<Vec<UsageMetric>, Status> {
        let response = self
            .0
            .clone()
            .get_usage(GetUsageRequest {
                start_time: start_time.to_rfc3339_opts(SecondsFormat::Secs, true),
                end_time: end_time.to_rfc3339_opts(SecondsFormat::Secs, true),
                project_id: project_id.map(ToString::to_string),
                collection,
            })
            .await?;
        Ok(response.into_inner().metrics)
    }
}
