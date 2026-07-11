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
    let media_type = req
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let bytes = Bytes::from_request(req, state)
        .await
        .map_err(|e| Error::BadRequest(format!("Failed to read request body: {e}")))?;

    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }

    let Some(media_type) = media_type else {
        return Err(Error::NotAcceptable("Missing Content-Type header".into()));
    };

    if !media_type.contains("json") {
        return Err(Error::UnsupportedMediaType(format!(
            "Content-Type header [{media_type}] is not supported"
        )));
    }

    // ES request bodies are always JSON objects. Reject arrays/scalars up front:
    // serde deserializes a struct from a positional sequence, so a bare `[]` would
    // otherwise become an all-defaults request (e.g. `_search` match-all).
    let json: serde_json::Value = serde_json::from_slice(&bytes)?;
    if !json.is_object() {
        return Err(Error::BadRequest("Request body must be a JSON object".into()));
    }
    Ok(Some(serde_json::from_value(json)?))
}
