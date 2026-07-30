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
            None => sonic_rs::from_slice(b"{}")?,
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
    // Classified before the body is consumed, but reported after: an empty body
    // is accepted whatever the header says.
    let media_type = MediaType::of(&req);

    let bytes = Bytes::from_request(req, state)
        .await
        .map_err(|e| Error::BadRequest(format!("Failed to read request body: {e}")))?;

    let body = bytes.trim_ascii();
    if body.is_empty() {
        return Ok(None);
    }

    match media_type {
        MediaType::Json => {}
        MediaType::Missing => {
            return Err(Error::NotAcceptable("Missing Content-Type header".into()))
        }
        MediaType::Other(media_type) => {
            return Err(Error::UnsupportedMediaType(format!(
                "Content-Type header [{media_type}] is not supported"
            )))
        }
    }

    // ES request bodies are always JSON objects. Reject arrays/scalars up front:
    // serde deserializes a struct from a positional sequence, so a bare `[]` would
    // otherwise become an all-defaults request (e.g. `_search` match-all).
    if !body.starts_with(b"{") {
        return Err(Error::BadRequest(
            "Request body must be a JSON object".into(),
        ));
    }

    Ok(Some(sonic_rs::from_slice(body)?))
}

enum MediaType {
    Json,
    Missing,
    Other(String),
}

impl MediaType {
    fn of(req: &Request) -> Self {
        match req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.is_empty())
        {
            None => MediaType::Missing,
            Some(media_type) if media_type.contains("json") => MediaType::Json,
            Some(media_type) => MediaType::Other(media_type.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body as AxumBody;
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Default, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Query {
        #[serde(default)]
        size: u64,
    }

    fn read(content_type: Option<&str>, body: &'static str) -> Result<Option<Query>, Error> {
        let mut req = Request::builder().method("POST");
        if let Some(content_type) = content_type {
            req = req.header(CONTENT_TYPE, content_type);
        }
        let req = req.body(AxumBody::from(body)).unwrap();

        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(read_json(req, &()))
    }

    #[rstest::rstest]
    #[case::object(r#"{"size": 5}"#)]
    #[case::surrounding_whitespace("\n  {\"size\": 5}\n")]
    fn reads_a_json_object(#[case] body: &'static str) {
        assert_eq!(
            read(Some("application/json"), body).unwrap(),
            Some(Query { size: 5 })
        );
    }

    // An empty body means "no body", whatever the headers say.
    #[rstest::rstest]
    #[case::empty("")]
    #[case::whitespace(" \n\t")]
    fn treats_an_empty_body_as_absent(#[case] body: &'static str) {
        assert_eq!(read(None, body).unwrap(), None);
    }

    // A struct deserializes from a positional sequence, so a bare array would
    // otherwise pass as an all-defaults request.
    #[rstest::rstest]
    #[case::array("[]")]
    #[case::array_of_values("[1, 2]")]
    #[case::scalar("5")]
    #[case::string(r#""a""#)]
    fn rejects_bodies_that_are_not_objects(#[case] body: &'static str) {
        let result = read(Some("application/json"), body);
        assert!(matches!(result, Err(Error::BadRequest(_))), "{result:?}");
    }

    #[test]
    fn requires_a_content_type() {
        assert!(matches!(read(None, "{}"), Err(Error::NotAcceptable(_))));
    }

    #[test]
    fn rejects_non_json_content_types() {
        assert!(matches!(
            read(Some("text/plain"), "{}"),
            Err(Error::UnsupportedMediaType(_))
        ));
    }

    #[test]
    fn reports_malformed_json() {
        assert!(matches!(
            read(Some("application/json"), "{\"size\":"),
            Err(Error::SerdeJson(_))
        ));
    }
}
