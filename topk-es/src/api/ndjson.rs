use std::marker::PhantomData;
use std::str::Lines;

use async_trait::async_trait;
use axum::extract::{FromRequest, FromRequestParts, Request};
use serde::de::DeserializeOwned;

use super::IndexName;
use crate::Error;

pub trait NdjsonHeader: DeserializeOwned {
    type Payload;

    fn index(&self) -> Option<IndexName>;

    fn parse_payload(self, lines: &mut NdjsonLines<'_>) -> Result<Self::Payload, Error>;
}

pub trait NdjsonJsonHeader: DeserializeOwned {
    type Payload: DeserializeOwned;

    fn index(&self) -> Option<IndexName>;
}

impl<H: NdjsonJsonHeader> NdjsonHeader for H {
    type Payload = H::Payload;

    fn index(&self) -> Option<IndexName> {
        H::index(self)
    }

    fn parse_payload(self, lines: &mut NdjsonLines<'_>) -> Result<Self::Payload, Error> {
        lines.parse()
    }
}

pub struct NdjsonBody<H: NdjsonHeader> {
    entries: Vec<(IndexName, H::Payload)>,
    _header: PhantomData<H>,
}

impl<H: NdjsonHeader> NdjsonBody<H> {
    pub fn into_entries(self) -> Vec<(IndexName, H::Payload)> {
        self.entries
    }

    fn parse(body: String, path: Option<IndexName>) -> Result<Self, Error> {
        if !body.ends_with('\n') {
            return Err(Error::BadRequest(
                "NDJSON request must be terminated by a newline [\\n]".into(),
            ));
        }

        let mut lines = NdjsonLines {
            lines: body.lines(),
        };

        let mut entries = Vec::new();
        while let Some(first) = lines.next() {
            let header: H = serde_json::from_str(first)?;
            let line_index = header.index();
            let payload = header.parse_payload(&mut lines)?;
            let index = line_index
                .or_else(|| path.clone())
                .ok_or_else(|| Error::BadRequest("Index is required".into()))?;
            entries.push((index, payload));
        }

        Ok(Self {
            entries,
            _header: PhantomData,
        })
    }
}

#[async_trait]
impl<H, S> FromRequest<S> for NdjsonBody<H>
where
    H: NdjsonHeader,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = req.into_parts();
        let path = Option::<IndexName>::from_request_parts(&mut parts, state)
            .await
            .expect("Option<IndexName> extraction is infallible");
        let body = String::from_request(Request::from_parts(parts, body), state)
            .await
            .map_err(|e| Error::BadRequest(format!("Failed to read NDJSON body: {e}")))?;
        Self::parse(body, path)
    }
}

pub struct NdjsonLines<'a> {
    lines: Lines<'a>,
}

impl<'a> NdjsonLines<'a> {
    fn next(&mut self) -> Option<&'a str> {
        self.lines.by_ref().find(|line| !line.trim().is_empty())
    }

    pub(crate) fn parse<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let line = self
            .next()
            .ok_or_else(|| Error::BadRequest("Unexpected end of NDJSON body".into()))?;

        Ok(serde_json::from_str(line)?)
    }
}
