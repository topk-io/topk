use async_trait::async_trait;
use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use http::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;

use crate::Error;

pub struct Body<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for Body<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let body = match read_json(req, state).await? {
            Some(body) => body,
            None => serde_json::from_str("{}")?,
        };
        Ok(Body(body))
    }
}

pub struct RequiredBody<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for RequiredBody<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        read_json(req, state)
            .await?
            .map(RequiredBody)
            .ok_or_else(|| Error::BadRequest("Request body is required".into()))
    }
}

async fn read_json<T, S>(req: Request, state: &S) -> Result<Option<T>, Error>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    let is_json = req
        .headers()
        .get(CONTENT_TYPE)
        .map(|value| value.to_str().unwrap_or_default().contains("json"));

    let bytes = Bytes::from_request(req, state)
        .await
        .map_err(|e| Error::BadRequest(format!("Failed to read request body: {e}")))?;

    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }

    if is_json == Some(false) {
        return Err(Error::UnsupportedMediaType(
            "Content-type must be application/json".into(),
        ));
    }

    Ok(Some(serde_json::from_slice(&bytes)?))
}
