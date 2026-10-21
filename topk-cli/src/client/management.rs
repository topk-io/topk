use anyhow::Result;
use tonic::transport::{ClientTlsConfig, Endpoint};

use crate::auth::Auth;
use crate::client::transport::Transport;
use crate::config::Config;
use crate::management::proto::collection_service_client::CollectionServiceClient;
use crate::management::proto::data_plane_service_client::DataPlaneServiceClient;
use crate::management::proto::project_service_client::ProjectServiceClient;
use crate::management::proto::region_service_client::RegionServiceClient;

#[derive(Clone)]
pub struct ManagementClient {
    pub projects: ProjectServiceClient<Transport>,
    pub collections: CollectionServiceClient<Transport>,
    pub regions: RegionServiceClient<Transport>,
    pub tokens: DataPlaneServiceClient<Transport>,
}

impl ManagementClient {
    pub fn new(config: Config) -> Result<Self> {
        let protocol = if config.https() { "https" } else { "http" };
        let mut grpc = Endpoint::from_shared(format!("{protocol}://api.{}", config.host()))?;
        if config.https() {
            grpc = grpc.tls_config(ClientTlsConfig::new().with_native_roots())?;
        }
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
