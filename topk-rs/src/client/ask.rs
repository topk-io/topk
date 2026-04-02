use futures_util::TryFutureExt;
use tokio_stream::StreamExt;
use tonic::Streaming;

use super::Response;
use crate::create_client;
use crate::proto::v1::ctx::ask_result::Answer;
use crate::proto::v1::ctx::ask_result::Message;
use crate::proto::v1::ctx::context_service_client::ContextServiceClient;
use crate::proto::v1::ctx::AskRequest;
use crate::proto::v1::ctx::AskResult;
use crate::proto::v1::ctx::Mode;
use crate::proto::v1::ctx::Source;
use crate::proto::v1::data::LogicalExpr;
use crate::retry::call_with_retry;
use crate::Error;

impl super::Client {
    pub async fn ask_stream(
        &self,
        query: impl Into<String>,
        sources: impl IntoIterator<Item = impl Into<Source>>,
        filter: Option<LogicalExpr>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> Result<Response<Streaming<AskResult>>, Error> {
        let client = create_client!(ContextServiceClient, self.channel, self.config).await?;

        let request = AskRequest {
            query: query.into(),
            sources: sources.into_iter().map(|s| s.into()).collect(),
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

    pub async fn ask(
        &self,
        query: impl Into<String>,
        sources: impl IntoIterator<Item = impl Into<Source>>,
        filter: Option<LogicalExpr>,
        mode: Option<Mode>,
        select_fields: Option<Vec<String>>,
    ) -> Result<Response<Answer>, Error> {
        let resp = self
            .ask_stream(query, sources, filter, mode, select_fields)
            .await?;
        let (mut stream, request_id) = resp.into_parts();

        let mut answer = None;
        while let Some(result) = stream.next().await {
            match result?.message {
                Some(Message::Answer(a)) => {
                    answer = Some(a);
                }
                Some(Message::Search(_)) => {}
                Some(Message::Reason(_)) => {}
                None => {
                    return Err(Error::Internal(
                        "Invalid proto: AskResult has no message".to_string(),
                    ))
                }
            }
        }

        match answer {
            Some(answer) => Ok(Response::new(answer, request_id)),
            None => Err(Error::Internal("No answer found".to_string())),
        }
    }
}
