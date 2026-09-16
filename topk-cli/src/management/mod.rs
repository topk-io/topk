use tonic::transport::Endpoint;

use crate::auth::Auth;
use crate::management::proto::collection_service_client::CollectionServiceClient;
use crate::management::proto::project_service_client::ProjectServiceClient;
use crate::management::proto::region_service_client::RegionServiceClient;

mod transport;
pub use transport::Transport;

pub mod proto {
    tonic::include_proto!("topk.management.v1");
}

#[derive(Clone)]
pub struct ManagementClient {
    pub projects: ProjectServiceClient<Transport>,
    pub collections: CollectionServiceClient<Transport>,
    pub regions: RegionServiceClient<Transport>,
}

impl ManagementClient {
    pub fn new(endpoint: Endpoint, auth: Auth) -> Self {
        let transport = Transport::new(endpoint.connect_lazy(), auth);
        Self {
            projects: ProjectServiceClient::new(transport.clone()),
            collections: CollectionServiceClient::new(transport.clone()),
            regions: RegionServiceClient::new(transport),
        }
    }
}
