use async_trait::async_trait;
use futures_util::TryFutureExt;
use tonic::Streaming;

use crate::proto::v1::ctx::AskRequest;
use crate::proto::v1::ctx::AskResponseMessage;
use crate::proto::v1::ctx::Effort;
use crate::proto::v1::ctx::Source;
use crate::proto::v1::data::LogicalExpr;
use crate::retry::call_with_retry;
use crate::Error;

#[async_trait]
pub trait AskExt {
    async fn ask(
        &self,
        query: String,
        sources: Vec<Source>,
        filter: Option<LogicalExpr>,
        effort: Effort,
    ) -> Result<Streaming<AskResponseMessage>, Error>;
}

#[async_trait]
impl AskExt for super::Client {
    async fn ask(
        &self,
        query: String,
        sources: Vec<Source>,
        filter: Option<LogicalExpr>,
        effort: Effort,
    ) -> Result<Streaming<AskResponseMessage>, Error> {
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
        .await
        .map_err(|e| match e {
            Error::NotFound => Error::DatasetNotFound,
            _ => e.into(),
        })?;

        Ok(response.into_inner())
    }
}
