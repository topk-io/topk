use tonic::transport::Endpoint;

use crate::auth::Auth;
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
    pub fn new(endpoint: Endpoint, auth: Auth) -> Self {
        let transport = Transport::new(endpoint.connect_lazy(), auth);
        Self {
            projects: ProjectServiceClient::new(transport.clone()),
            collections: CollectionServiceClient::new(transport.clone()),
            regions: RegionServiceClient::new(transport.clone()),
            tokens: DataPlaneServiceClient::new(transport),
        }
    }
}
