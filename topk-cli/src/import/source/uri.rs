use std::fmt;
use std::str::FromStr;

use url::Url;

use crate::import::error::Error;
use crate::ProjectId;

use super::duck::Duckdb;
use super::es::Es;
use super::mongo::Mongo;
use super::File;
use super::{Backend, Source};

/// A source as named on the command line, parsed but not connected.
#[derive(Clone)]
pub enum Uri {
    Postgres(Url),
    Mysql(Url),
    Sqlite(String),
    Files(Option<File>),
    Elasticsearch(Url),
    Mongo(Url),
    Topk {
        project: Option<ProjectId>,
        collection: String,
    },
}

impl Uri {
    pub async fn connect(&self) -> Result<Source, Error> {
        let locator = self.to_string();
        let backend = match self {
            Uri::Postgres(url) => Backend::Duck(Duckdb::Postgres(url.clone())),
            Uri::Mysql(url) => Backend::Duck(Duckdb::Mysql(url.clone())),
            Uri::Sqlite(path) => Backend::Duck(Duckdb::Sqlite(path.clone())),
            Uri::Files(file) => Backend::Duck(Duckdb::Files(file.clone())),
            Uri::Elasticsearch(url) => Backend::Es(Es::new(url.clone())?),
            Uri::Mongo(url) => Backend::Mongo(Mongo::connect(url).await?),
            Uri::Topk { .. } => {
                return Err(Error::InvalidArgument(
                    "TopK sources require a connected client; use Source::topk".into(),
                ))
            }
        };
        Ok(Source {
            locator,
            backend,
            catalog: tokio::sync::OnceCell::new(),
        })
    }

    pub fn password(&self) -> Option<&str> {
        match self {
            Uri::Postgres(url) | Uri::Mysql(url) | Uri::Elasticsearch(url) | Uri::Mongo(url) => {
                url.password()
            }
            _ => None,
        }
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
            if rest.contains('@') {
                return Err(Error::InvalidArgument(
                    "topk:// does not take an API key; use --project-id".into(),
                ));
            }
            let (collection, project_id) = match rest.split_once('/') {
                Some((project, collection)) => (
                    collection,
                    Some(project.parse::<ProjectId>().map_err(|_| {
                        Error::InvalidArgument(format!("invalid project ID: {project:?}"))
                    })?),
                ),
                None => (rest, None),
            };
            if collection.contains('/') {
                return Err(Error::InvalidArgument(
                    "topk:// collection names cannot contain '/'".into(),
                ));
            }
            if collection.is_empty() || collection.contains(['*', '?']) {
                return Err(Error::InvalidArgument("topk:// needs an exact collection name, e.g. topk://books or topk://project/books".into()));
            }
            return Ok(Uri::Topk {
                collection: collection.to_string(),
                project: project_id,
            });
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
                return Ok(Uri::Elasticsearch(url(&format!("{scheme}://{rest}"))?));
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
                return Ok(Uri::Elasticsearch(url));
            }
        }
        if s.starts_with("postgres://") || s.starts_with("postgresql://") {
            return Ok(Uri::Postgres(url(s)?));
        }
        if s.starts_with("mysql://") || s.starts_with("mariadb://") {
            return Ok(Uri::Mysql(url(s)?));
        }
        if let Some(path) = s
            .strip_prefix("sqlite://")
            .or_else(|| s.strip_prefix("sqlite:"))
        {
            return Ok(Uri::Sqlite(
                std::path::absolute(&*shellexpand::tilde(path))?
                    .display()
                    .to_string(),
            ));
        }
        Ok(Uri::Files(Some(s.parse()?)))
    }
}

/// The uri without its password — what a state file or message may carry.
impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Uri::Postgres(url) | Uri::Mysql(url) => f.write_str(&redact(url)),
            Uri::Sqlite(path) => write!(f, "sqlite:{path}"),
            Uri::Files(Some(file)) => f.write_str(&file.path),
            Uri::Files(None) => Ok(()),
            Uri::Elasticsearch(url) => write!(f, "es+{}", redact(url)),
            Uri::Mongo(url) => f.write_str(&redact(url)),
            Uri::Topk {
                project: Some(project),
                collection,
            } => write!(f, "topk://{project}/{collection}"),
            Uri::Topk {
                project: None,
                collection,
            } => write!(f, "topk://{collection}"),
        }
    }
}

impl fmt::Debug for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

fn url(s: &str) -> Result<Url, Error> {
    Url::parse(s).map_err(|error| {
        Error::Setup(anyhow::Error::new(error).context(format!(
            "bad source uri {:?}",
            if s.contains('@') { "<redacted>" } else { s }
        )))
    })
}

pub(super) fn redact(url: &Url) -> String {
    if url.password().is_none() {
        return url.to_string();
    }
    let mut url = url.clone();
    let _ = url.set_password(Some("***"));
    url.to_string()
}
