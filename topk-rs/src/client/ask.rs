use futures_util::TryFutureExt;
use tonic::Streaming;

use super::Response;
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
        sources: impl IntoIterator<Item = impl Into<Source>>,
        filter: Option<LogicalExpr>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> Result<Response<Streaming<AskResult>>, Error> {
        let client = super::create_ctx_client(&self.config(), &self.channel()).await?;

        let request = AskRequest {
            query: query.into(),
            sources: sources.into_iter().map(|s| s.into()).collect(),
            filter,
            mode: mode.unwrap_or(Mode::Reason) as i32,
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
