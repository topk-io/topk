use futures_util::TryFutureExt;
use tonic::Streaming;

use crate::proto::v1::ctx::AskRequest;
use crate::proto::v1::ctx::AskResponseMessage;
use crate::proto::v1::ctx::Effort;
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
        effort: Option<Effort>,
    ) -> Result<Streaming<AskResponseMessage>, Error> {
        let query = query.into();
        let sources: Vec<_> = sources.into_iter().map(|s| s.into()).collect();
        let filter = filter.clone();
        let effort = effort.unwrap_or(Effort::Unspecified);
        let client = super::create_ctx_client(&self.config(), &self.channel()).await?;

        let response = call_with_retry(&self.config().retry_config(), || {
            let mut client = client.clone();
            let query = query.clone();
            let sources = sources.clone();
            let filter = filter.clone();

            async move {
                client
                    .ask(AskRequest {
                        query,
                        sources,
                        filter,
                        effort: effort as i32,
                    })
                    .map_err(Error::from)
                    .await
            }
        })
        .await?;

        Ok(response.into_inner())
    }
}
