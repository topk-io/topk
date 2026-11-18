use std::fmt;
use std::str::FromStr;

use percent_encoding::{
    percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS, NON_ALPHANUMERIC,
};
use url::Url;

const ESCAPE: &AsciiSet = &CONTROLS.add(b' ').add(b'%').add(b'#');

use crate::import::error::Error;
use crate::ProjectId;

use super::duck::Duckdb;
use super::es::Es;
use super::mongo::Mongo;
use super::File;
use super::{Backend, Source};

/// A source as named on the command line, parsed but not connected.
#[derive(Clone, Debug)]
pub enum Uri {
    Postgres {
        url: SourceUrl,
        object: String,
    },
    Mysql {
        url: SourceUrl,
        object: String,
    },
    Sqlite {
        path: String,
        object: String,
    },
    Files(File),
    Elasticsearch {
        url: SourceUrl,
        index: String,
    },
    Mongo {
        url: SourceUrl,
        object: String,
    },
    Topk {
        project: Option<ProjectId>,
        collection: String,
    },
}

impl Uri {
    pub async fn connect(&self) -> Result<Source, Error> {
        let locator = self.to_string();
        let backend = match self {
            Uri::Postgres { url, object } => Backend::Duck(Duckdb::Postgres {
                url: url.clone(),
                object: object.clone(),
            }),
            Uri::Mysql { url, object } => Backend::Duck(Duckdb::Mysql {
                url: url.clone(),
                object: object.clone(),
            }),
            Uri::Sqlite { path, object } => Backend::Duck(Duckdb::Sqlite {
                path: path.clone(),
                object: object.clone(),
            }),
            Uri::Files(file) => Backend::Duck(Duckdb::Files(file.clone())),
            Uri::Elasticsearch { url, index } => {
                Backend::Es(Es::new(url.0.clone(), index.clone())?)
            }
            Uri::Mongo { url, object } => {
                Backend::Mongo(Mongo::connect(&url.0, object.clone()).await?)
            }
            Uri::Topk { .. } => {
                return Err(Error::InvalidArgument(
                    "TopK sources require a connected client; use Source::topk".into(),
                ))
            }
        };
        Ok(Source {
            locator,
            backend,
            table: tokio::sync::OnceCell::new(),
        })
    }

    pub fn password(&self) -> Option<&str> {
        match self {
            Uri::Postgres { url, .. }
            | Uri::Mysql { url, .. }
            | Uri::Elasticsearch { url, .. }
            | Uri::Mongo { url, .. } => url.password(),
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
        let decode = |value: &str| -> Result<String, Error> {
            if value.as_bytes().iter().enumerate().any(|(i, b)| {
                *b == b'%'
                    && !value
                        .as_bytes()
                        .get(i + 1..i + 3)
                        .is_some_and(|bytes| bytes.iter().all(u8::is_ascii_hexdigit))
            }) {
                return Err(Error::InvalidArgument(
                    "invalid percent encoding in source path or object".into(),
                ));
            }
            percent_decode_str(value)
                .decode_utf8()
                .map(|value| value.into_owned())
                .map_err(|_| Error::InvalidArgument("source path or object is not UTF-8".into()))
        };
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
        let elastic = [
            ("elasticsearch://", "http"),
            ("elasticsearch+http://", "http"),
            ("elasticsearch+https://", "https"),
            ("es://", "http"),
            ("es+http://", "http"),
            ("es+https://", "https"),
        ]
        .into_iter()
        .find_map(|(prefix, scheme)| {
            s.strip_prefix(prefix)
                .map(|rest| format!("{scheme}://{rest}"))
        });
        let endpoint = match elastic.as_deref() {
            Some(input) => Some(url(input)?),
            None if s.starts_with("http://") || s.starts_with("https://") => {
                let url = url(s)?;
                url.host_str()
                    .is_some_and(|host| {
                        host.ends_with(".cloud.es.io") || host.ends_with(".elastic.cloud")
                    })
                    .then_some(url)
            }
            None => None,
        };
        if let Some(mut url) = endpoint {
            let index = decode(url.path().trim_matches('/'))?;
            if index.is_empty()
                || index.contains(['*', '?', ',', '/'])
                || index == "_all"
                || url.fragment().is_some()
            {
                return Err(Error::InvalidArgument(
                    "elasticsearch source requires one index, e.g. es://host/orders".into(),
                ));
            }
            url.set_path("/");
            return Ok(Uri::Elasticsearch {
                url: SourceUrl(url),
                index,
            });
        }
        if let Some(path) = s
            .strip_prefix("sqlite://")
            .or_else(|| s.strip_prefix("sqlite:"))
        {
            let (path, object) = path
                .split_once('#')
                .filter(|(_, object)| !object.is_empty())
                .ok_or_else(|| {
                    Error::InvalidArgument(format!(
                        "sqlite source requires #<table>, e.g. sqlite:{path}#orders"
                    ))
                })?;
            return Ok(Uri::Sqlite {
                path: std::path::absolute(&*shellexpand::tilde(&decode(path)?))?
                    .display()
                    .to_string(),
                object: decode(object)?,
            });
        }
        if matches!(
            s.split_once("://").map(|(scheme, _)| scheme),
            Some("postgres" | "postgresql" | "mysql" | "mariadb" | "mongodb" | "mongodb+srv")
        ) {
            let input = if matches!(s.split('#').next(), Some("mongodb://" | "mongodb+srv://")) {
                let connection = std::env::var("MONGODB_URI").map_err(|_| {
                    Error::InvalidArgument(
                        "bare mongodb:// needs MONGODB_URI set, or pass the full URL".into(),
                    )
                })?;
                match s.split_once('#') {
                    Some((_, object)) => format!(
                        "{}#{object}",
                        connection.split('#').next().unwrap_or(&connection)
                    ),
                    None => connection,
                }
            } else {
                s.to_string()
            };
            let mut url = url(&input)?;
            if matches!(url.scheme(), "mongodb" | "mongodb+srv")
                && url.path().trim_matches('/').is_empty()
            {
                return Err(Error::InvalidArgument(
                    "mongodb url must include a database, e.g. mongodb://host/dbname#collection"
                        .into(),
                ));
            }
            let object = decode(
                url.fragment()
                    .filter(|object| !object.is_empty())
                    .ok_or_else(|| {
                        let (object, sample) = match url.scheme() {
                            "mongodb" | "mongodb+srv" => ("collection", "products"),
                            _ => ("table", "orders"),
                        };
                        Error::InvalidArgument(format!(
                            "{scheme} source requires #<{object}>, e.g. {scheme}://host/db#{sample}",
                            scheme = url.scheme()
                        ))
                    })?,
            )?;
            url.set_fragment(None);
            return Ok(match url.scheme() {
                "postgres" | "postgresql" => Uri::Postgres {
                    url: SourceUrl(url),
                    object,
                },
                "mysql" | "mariadb" => Uri::Mysql {
                    url: SourceUrl(url),
                    object,
                },
                _ => Uri::Mongo {
                    url: SourceUrl(url),
                    object,
                },
            });
        }
        Ok(Uri::Files(s.parse()?))
    }
}

/// The uri without its password — what a state file or message may carry.
impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Uri::Postgres { url, object }
            | Uri::Mysql { url, object }
            | Uri::Mongo { url, object } => {
                write!(f, "{url}#{}", utf8_percent_encode(object, ESCAPE))
            }
            Uri::Sqlite { path, object } => write!(
                f,
                "sqlite:{}#{}",
                utf8_percent_encode(path, ESCAPE),
                utf8_percent_encode(object, ESCAPE)
            ),
            Uri::Files(file) => f.write_str(&file.path),
            Uri::Elasticsearch { url, index } => {
                let mut url = url.0.clone();
                url.set_path(&format!(
                    "/{}",
                    utf8_percent_encode(index, NON_ALPHANUMERIC)
                ));
                write!(f, "es+{}", SourceUrl(url))
            }
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

/// A source url whose password prints as `***`, in the userinfo or as a
/// `password` / `sslpassword` query parameter.
#[derive(Clone)]
pub struct SourceUrl(pub Url);

impl SourceUrl {
    pub fn password(&self) -> Option<&str> {
        self.0.password().or_else(|| {
            self.0
                .query()?
                .split('&')
                .filter_map(|pair| pair.split_once('='))
                .find(|(key, _)| secret(key))
                .map(|(_, value)| value)
        })
    }
}

impl fmt::Display for SourceUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.password().is_none() {
            return write!(f, "{}", self.0);
        }
        let mut url = self.0.clone();
        if url.password().is_some() {
            let _ = url.set_password(Some("***"));
        }
        if let Some(query) = self.0.query() {
            let query: Vec<_> = query
                .split('&')
                .map(|pair| match pair.split_once('=') {
                    Some((key, _)) if secret(key) => format!("{key}=***"),
                    _ => pair.to_string(),
                })
                .collect();
            url.set_query(Some(&query.join("&")));
        }
        write!(f, "{url}")
    }
}

impl fmt::Debug for SourceUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

fn url(s: &str) -> Result<Url, Error> {
    Url::parse(s).map_err(|error| {
        Error::Setup(anyhow::Error::new(error).context(format!(
            "bad source uri {:?}",
            if s.contains('@') || s.to_ascii_lowercase().contains("password=") {
                "<redacted>"
            } else {
                s
            }
        )))
    })
}

fn secret(key: &str) -> bool {
    key.eq_ignore_ascii_case("password") || key.eq_ignore_ascii_case("sslpassword")
}
