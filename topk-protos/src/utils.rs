use crate::{
    control::v1::collection_service_client::CollectionServiceClient,
    data::v1::{
        query_service_client::QueryServiceClient, write_service_client::WriteServiceClient,
    },
};
use std::collections::HashMap;
use std::str::FromStr;
use tonic::{
    metadata::AsciiMetadataValue,
    service::{interceptor::InterceptedService, Interceptor},
    transport::Channel,
    Status,
};

pub struct AppendHeadersInterceptor {
    headers: HashMap<&'static str, String>,
}

impl Interceptor for AppendHeadersInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        for (key, value) in self.headers.clone().into_iter() {
            request.metadata_mut().insert(
                key,
                AsciiMetadataValue::from_str(value.as_str()).expect("invalid header value"),
            );
        }

        Ok(request)
    }
}

pub type WriteClient = WriteServiceClient<Channel>;
pub type WriteClientWithHeaders =
    WriteServiceClient<InterceptedService<Channel, AppendHeadersInterceptor>>;

impl WriteClient {
    pub fn with_headers(
        channel: Channel,
        headers: HashMap<&'static str, String>,
    ) -> WriteClientWithHeaders {
        Self::with_interceptor(channel, AppendHeadersInterceptor { headers })
    }
}

pub type QueryClient = QueryServiceClient<Channel>;
pub type QueryClientWithHeaders =
    QueryServiceClient<InterceptedService<Channel, AppendHeadersInterceptor>>;

impl QueryClient {
    pub fn with_headers(
        channel: Channel,
        headers: HashMap<&'static str, String>,
    ) -> QueryClientWithHeaders {
        Self::with_interceptor(channel, AppendHeadersInterceptor { headers })
    }
}

pub type CollectionClient = CollectionServiceClient<Channel>;
pub type CollectionClientWithHeaders =
    CollectionServiceClient<InterceptedService<Channel, AppendHeadersInterceptor>>;

impl CollectionClient {
    pub fn with_headers(
        channel: Channel,
        headers: HashMap<&'static str, String>,
    ) -> CollectionClientWithHeaders {
        Self::with_interceptor(channel, AppendHeadersInterceptor { headers })
    }
}
