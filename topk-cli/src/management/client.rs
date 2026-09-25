use anyhow::Result;
use tonic::transport::{ClientTlsConfig, Endpoint};

use crate::auth::Auth;
use crate::config::Config;
use crate::endpoint::{Host, ManagementEndpoint};
use crate::management::proto::collection_service_client::CollectionServiceClient;
use crate::management::proto::data_plane_service_client::DataPlaneServiceClient;
use crate::management::proto::project_service_client::ProjectServiceClient;
use crate::management::proto::region_service_client::RegionServiceClient;
use crate::management::Transport;

#[derive(Clone)]
pub struct Client {
    pub projects: ProjectServiceClient<Transport>,
    pub collections: CollectionServiceClient<Transport>,
    pub regions: RegionServiceClient<Transport>,
    pub tokens: DataPlaneServiceClient<Transport>,
}

impl Client {
    pub fn new(endpoint: ManagementEndpoint) -> Result<Self> {
        let Host { host, https } = &endpoint.host;
        let protocol = if *https { "https" } else { "http" };
        let mut grpc = Endpoint::from_shared(format!("{protocol}://api.{host}"))?;
        if *https {
            grpc = grpc.tls_config(ClientTlsConfig::new().with_native_roots())?;
        }
        let config = Config::new(endpoint.oauth, Config::dir()?);
        Ok(Self::connect(grpc, Auth::new(config)?))
    }

    pub fn connect(grpc: Endpoint, auth: Auth) -> Self {
        let transport = Transport::new(grpc.connect_lazy(), auth);
        Self {
            projects: ProjectServiceClient::new(transport.clone()),
            collections: CollectionServiceClient::new(transport.clone()),
            regions: RegionServiceClient::new(transport.clone()),
            tokens: DataPlaneServiceClient::new(transport),
        }
    }
}
