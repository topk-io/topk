use futures_util::TryFutureExt;
use tonic::Streaming;

use super::Response;
use crate::proto::v1::ctx::SearchRequest;
use crate::proto::v1::ctx::SearchResult;
use crate::proto::v1::ctx::Source;
use crate::proto::v1::data::LogicalExpr;
use crate::retry::call_with_retry;
use crate::Error;

impl super::Client {
    pub async fn search(
        &self,
        query: impl Into<String>,
        sources: impl IntoIterator<Item = impl Into<Source>>,
        top_k: u32,
        filter: Option<LogicalExpr>,
        select_fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Response<Streaming<SearchResult>>, Error> {
        let client = super::create_ctx_client(&self.config(), &self.channel()).await?;

        let request = SearchRequest {
            query: query.into(),
            sources: sources.into_iter().map(|s| s.into()).collect(),
            filter,
            top_k,
            select_fields: select_fields.into_iter().map(|s| s.into()).collect(),
        };

        let response = call_with_retry(&self.config().retry_config(), || {
            let request = request.clone();
            let mut client = client.clone();
            async move { client.search(request).map_err(Error::from).await }
        })
        .await?;

        Ok(response.into())
    }
}
