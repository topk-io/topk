use std::marker::PhantomData;
use std::slice::Split;

use async_trait::async_trait;
use axum::body::Bytes;
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

    fn parse(body: &[u8], path: Option<IndexName>) -> Result<Self, Error> {
        if !body.ends_with(b"\n") {
            return Err(Error::BadRequest(
                "NDJSON request must be terminated by a newline [\\n]".into(),
            ));
        }

        let mut lines = NdjsonLines {
            lines: body.split(is_newline),
        };

        let mut entries = Vec::new();
        while let Some(first) = lines.next() {
            let header: H = sonic_rs::from_slice(first)?;
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
        let body = Bytes::from_request(Request::from_parts(parts, body), state)
            .await
            .map_err(|e| Error::BadRequest(format!("Failed to read NDJSON body: {e}")))?;
        Self::parse(&body, path)
    }
}

fn is_newline(byte: &u8) -> bool {
    *byte == b'\n'
}

pub struct NdjsonLines<'a> {
    lines: Split<'a, u8, fn(&u8) -> bool>,
}

impl<'a> NdjsonLines<'a> {
    fn next(&mut self) -> Option<&'a [u8]> {
        self.lines
            .by_ref()
            .map(|line| line.trim_ascii())
            .find(|line| !line.is_empty())
    }

    pub(crate) fn parse<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let line = self
            .next()
            .ok_or_else(|| Error::BadRequest("Unexpected end of NDJSON body".into()))?;

        Ok(sonic_rs::from_slice(line)?)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Header {
        #[serde(default)]
        index: Option<IndexName>,
    }

    impl NdjsonJsonHeader for Header {
        type Payload = Vec<i64>;

        fn index(&self) -> Option<IndexName> {
            self.index.clone()
        }
    }

    fn parse(body: &str, path: Option<&str>) -> Result<Vec<(IndexName, Vec<i64>)>, Error> {
        let path = path.map(|index| IndexName::try_from(index.to_string()).unwrap());
        NdjsonBody::<Header>::parse(body.as_bytes(), path).map(NdjsonBody::into_entries)
    }

    #[test]
    fn pairs_each_header_with_its_payload() {
        let entries = parse("{\"index\":\"a\"}\n[1,2]\n{\"index\":\"b\"}\n[3]\n", None).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0.as_str(), "a");
        assert_eq!(entries[0].1, vec![1, 2]);
        assert_eq!(entries[1].0.as_str(), "b");
        assert_eq!(entries[1].1, vec![3]);
    }

    #[test]
    fn skips_blank_lines_and_carriage_returns() {
        let entries = parse("{}\r\n[1]\r\n\r\n{}\n\n[2]\n", Some("idx")).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0.as_str(), "idx");
        assert_eq!(entries[0].1, vec![1]);
        assert_eq!(entries[1].1, vec![2]);
    }

    #[test]
    fn requires_a_trailing_newline() {
        assert!(parse("{}\n[1]", Some("idx")).is_err());
    }

    #[test]
    fn requires_an_index() {
        assert!(parse("{}\n[1]\n", None).is_err());
    }

    #[test]
    fn requires_a_payload_for_every_header() {
        assert!(parse("{}\n", Some("idx")).is_err());
    }
}
