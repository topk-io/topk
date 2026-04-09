use futures_util::TryFutureExt;
use tonic::Streaming;

use super::Response;
use crate::create_client;
use crate::proto::v1::ctx::context_service_client::ContextServiceClient;
use crate::proto::v1::ctx::AskRequest;
use crate::proto::v1::ctx::AskResult;
use crate::proto::v1::ctx::Mode;
use crate::proto::v1::ctx::Source;
use crate::proto::v1::data::LogicalExpr;
use crate::retry::call_with_retry;
use crate::Error;

impl super::Client {
    pub async fn ask(
        &self,
        query: impl Into<String>,
        datasets: impl IntoIterator<Item = impl Into<Source>>,
        filter: Option<LogicalExpr>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> Result<Response<Streaming<AskResult>>, Error> {
        let datasets: Vec<_> = datasets.into_iter().map(|s| s.into()).collect();
        if datasets.is_empty() {
            return Err(Error::InvalidArgument(
                "provide at least one dataset".to_string(),
            ));
        }

        let client = create_client!(ContextServiceClient, self.channel, self.config).await?;

        let request = AskRequest {
            query: query.into(),
            datasets,
            filter,
            mode: mode.unwrap_or(Mode::Auto).into(),
            select_fields: select_fields.unwrap_or_default(),
        };

        let response = call_with_retry(&self.config().retry_config(), || {
            let request = request.clone();
            let mut client = client.clone();
            async move { client.ask(request).map_err(Error::from).await }
        })
        .await?;

        Ok(response.into())
    }
}
