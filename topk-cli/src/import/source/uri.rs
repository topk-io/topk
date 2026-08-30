use std::fmt;
use std::str::FromStr;

use url::Url;

use crate::import::error::Error;

use super::duck::Duckdb;
use super::topk;

/// A source as named on the command line, parsed but not connected.
#[derive(Clone)]
pub enum Uri {
    Duck(Duckdb),
    Es(Url),
    Mongo(Url),
    Topk(topk::Uri),
}

/// No source: every spec `from` is its own file locator.
impl Default for Uri {
    fn default() -> Uri {
        Uri::Duck(Duckdb::Files(None))
    }
}

impl FromStr for Uri {
    type Err = Error;

    fn from_str(s: &str) -> Result<Uri, Error> {
        let s = s.trim();
        if s.is_empty() {
            return Err(Error::InvalidArgument("empty source uri".into()));
        }
        if let Some(rest) = s.strip_prefix("topk://") {
            let (authority, collection) = rest.split_once('/').unwrap_or((rest, ""));
            let (api_key, region) = match authority.rsplit_once('@') {
                Some((key, region)) => (Some(key.to_string()), region),
                None => (None, authority),
            };
            if region.is_empty() {
                return Err(Error::InvalidArgument(
                    "topk:// needs a region, e.g. topk://sunflower/books".into(),
                ));
            }
            return Ok(Uri::Topk(topk::Uri {
                region: region.to_string(),
                api_key,
                collection: collection.to_string(),
            }));
        }
        if s.starts_with("mongodb://") || s.starts_with("mongodb+srv://") {
            // A bare scheme reads MONGODB_URI.
            let s = match s {
                "mongodb://" | "mongodb+srv://" => std::env::var("MONGODB_URI").map_err(|_| {
                    Error::InvalidArgument(
                        "bare mongodb:// needs MONGODB_URI set, or pass the full URL".into(),
                    )
                })?,
                _ => s.to_string(),
            };
            let url = url(&s)?;
            if url
                .path_segments()
                .and_then(|mut p| p.next())
                .filter(|p| !p.is_empty())
                .is_none()
            {
                return Err(Error::InvalidArgument(
                    "mongodb url must include a database, e.g. mongodb://host/dbname".into(),
                ));
            }
            return Ok(Uri::Mongo(url));
        }
        for (prefix, scheme) in [
            ("elasticsearch://", "http"),
            ("elasticsearch+http://", "http"),
            ("elasticsearch+https://", "https"),
            ("es://", "http"),
            ("es+http://", "http"),
            ("es+https://", "https"),
        ] {
            if let Some(rest) = s.strip_prefix(prefix) {
                return Ok(Uri::Es(url(&format!("{scheme}://{rest}"))?));
            }
        }
        // An Elastic Cloud endpoint is ES even as a bare https url:
        // hosted is <deployment>.es.<region>.<provider>.cloud.es.io,
        // serverless is <project>.es.<region>.<provider>.elastic.cloud.
        if s.starts_with("http://") || s.starts_with("https://") {
            let url = url(s)?;
            if url.host_str().is_some_and(|host| {
                host.ends_with(".cloud.es.io") || host.ends_with(".elastic.cloud")
            }) {
                return Ok(Uri::Es(url));
            }
        }
        if s.starts_with("postgres://") || s.starts_with("postgresql://") {
            return Ok(Uri::Duck(Duckdb::Postgres(url(s)?)));
        }
        if s.starts_with("mysql://") || s.starts_with("mariadb://") {
            return Ok(Uri::Duck(Duckdb::Mysql(url(s)?)));
        }
        if let Some(path) = s
            .strip_prefix("sqlite://")
            .or_else(|| s.strip_prefix("sqlite:"))
        {
            return Ok(Uri::Duck(Duckdb::Sqlite(
                shellexpand::tilde(path).into_owned(),
            )));
        }
        Ok(Uri::Duck(Duckdb::Files(Some(s.parse()?))))
    }
}

/// The uri without its password — what a state file or message may carry.
impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Uri::Duck(duck) => duck.fmt(f),
            Uri::Es(url) | Uri::Mongo(url) => f.write_str(&redact(url)),
            Uri::Topk(uri) => uri.fmt(f),
        }
    }
}

impl fmt::Debug for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Uri::Duck(duck) => duck.fmt(f),
            Uri::Es(url) => f.debug_tuple("Es").field(&redact(url)).finish(),
            Uri::Mongo(url) => f.debug_tuple("Mongo").field(&redact(url)).finish(),
            Uri::Topk(uri) => uri.fmt(f),
        }
    }
}

fn url(s: &str) -> Result<Url, Error> {
    Url::parse(s).map_err(|e| Error::InvalidArgument(format!("bad source uri {s:?}: {e}")))
}

pub(super) fn redact(url: &Url) -> String {
    if url.password().is_none() {
        return url.to_string();
    }
    let mut url = url.clone();
    let _ = url.set_password(Some("***"));
    url.to_string()
}
