use std::path::Path;
use std::str::FromStr;

use url::Url;

use crate::import::error::Error;

#[derive(Debug, Clone)]
pub enum ObjectStore {
    S3,
    Gcs,
    Azure,
    HuggingFace,
    /// Plain http(s) — read through httpfs, no credentials.
    Http,
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Csv,
    Json,
    Parquet,
    Arrow,
    Avro,
    Xlsx,
}

#[derive(Clone)]
pub enum Uri {
    Postgres(Url),
    Mysql(Url),
    Sqlite(String),
    Elasticsearch(Url),
    Mongo(Url),
    File {
        path: String,
        store: Option<ObjectStore>,
        format: Format,
    },
}

// Hand-written so a password in a DSN cannot reach a log line or an error.
impl std::fmt::Debug for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Uri::Postgres(url) => f.debug_tuple("Postgres").field(&redact(url)).finish(),
            Uri::Mysql(url) => f.debug_tuple("Mysql").field(&redact(url)).finish(),
            Uri::Elasticsearch(url) => f.debug_tuple("Elasticsearch").field(&redact(url)).finish(),
            Uri::Mongo(url) => f.debug_tuple("Mongo").field(&redact(url)).finish(),
            Uri::Sqlite(path) => f.debug_tuple("Sqlite").field(path).finish(),
            Uri::File {
                path,
                store,
                format,
            } => f
                .debug_struct("File")
                .field("path", path)
                .field("store", store)
                .field("format", format)
                .finish(),
        }
    }
}

impl Uri {
    /// The uri without its password — what a state file or message may carry.
    pub fn redacted(&self) -> String {
        match self {
            Uri::Postgres(url) | Uri::Mysql(url) | Uri::Elasticsearch(url) | Uri::Mongo(url) => {
                redact(url)
            }
            Uri::Sqlite(path) => format!("sqlite:{path}"),
            Uri::File { path, .. } => path.clone(),
        }
    }
}

fn redact(url: &Url) -> String {
    if url.password().is_none() {
        return url.to_string();
    }
    let mut url = url.clone();
    let _ = url.set_password(Some("***"));
    url.to_string()
}

impl FromStr for Uri {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(Error::InvalidArgument("empty source uri".into()));
        }

        if s.starts_with("postgres://") || s.starts_with("postgresql://") {
            return Ok(Self::Postgres(parse_url(s)?));
        }
        if s.starts_with("mysql://") || s.starts_with("mariadb://") {
            return Ok(Self::Mysql(parse_url(s)?));
        }
        if let Some(path) = s
            .strip_prefix("sqlite://")
            .or_else(|| s.strip_prefix("sqlite:"))
        {
            return Ok(Self::Sqlite(shellexpand::tilde(path).into_owned()));
        }
        if s == "mongodb://" || s == "mongodb+srv://" {
            return match std::env::var("MONGODB_URI") {
                Ok(uri) => uri.parse(),
                Err(_) => Err(Error::InvalidArgument(
                    "bare mongodb:// needs MONGODB_URI set, or pass the full URL".into(),
                )),
            };
        }
        if s.starts_with("mongodb://") || s.starts_with("mongodb+srv://") {
            let url = parse_url(s)?;
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
            return Ok(Self::Mongo(url));
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
                return Ok(Self::Elasticsearch(parse_url(&format!(
                    "{scheme}://{rest}"
                ))?));
            }
        }
        // An Elastic Cloud endpoint is ES even as a bare https url:
        // hosted is <deployment>.es.<region>.<provider>.cloud.es.io,
        // serverless is <project>.es.<region>.<provider>.elastic.cloud.
        if s.starts_with("http://") || s.starts_with("https://") {
            let url = parse_url(s)?;
            if url.host_str().is_some_and(|host| {
                host.ends_with(".cloud.es.io") || host.ends_with(".elastic.cloud")
            }) {
                return Ok(Self::Elasticsearch(url));
            }
        }

        let store = if s.starts_with("s3://") || s.starts_with("r2://") {
            Some(ObjectStore::S3)
        } else if s.starts_with("gs://") || s.starts_with("gcs://") {
            Some(ObjectStore::Gcs)
        } else if s.starts_with("az://") || s.starts_with("azure://") {
            Some(ObjectStore::Azure)
        } else if s.starts_with("hf://") {
            Some(ObjectStore::HuggingFace)
        } else if s.starts_with("http://") || s.starts_with("https://") {
            Some(ObjectStore::Http)
        } else {
            None
        };

        // Only http: elsewhere `?` is a glob character.
        let path = match store {
            Some(ObjectStore::Http) => s.split(['?', '#']).next().unwrap_or(s),
            _ => s,
        };
        // duckdb reads compressed csv/json transparently; type data.csv.gz by
        // its inner extension.
        let path = match extension(Path::new(path)).as_deref() {
            Some("gz" | "zst") => Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(path),
            _ => path,
        };
        let format = match extension(Path::new(path)).as_deref() {
            Some("csv" | "tsv") => Format::Csv,
            Some("json" | "jsonl" | "ndjson") => Format::Json,
            Some("parquet") => Format::Parquet,
            Some("arrow") => Format::Arrow,
            Some("avro") => Format::Avro,
            Some("xlsx") => Format::Xlsx,
            other => {
                let hint = match store {
                    Some(ObjectStore::Http) => {
                        ". For Elasticsearch, use elasticsearch://host or elasticsearch+https://host"
                    }
                    // A dataset repo is a tree of files, not a file; name one.
                    Some(ObjectStore::HuggingFace) => {
                        ". A hugging face dataset holds <config>/<split>-*.parquet files, \
                         or read hugging face's converted copy at \
                         hf://datasets/<org>/<name>@~parquet/<config>/<split>/*.parquet"
                    }
                    _ if matches!(other, Some("db" | "sqlite" | "sqlite3")) => {
                        ". For SQLite, use sqlite:<path>"
                    }
                    _ => "",
                };
                return Err(Error::InvalidArgument(format!(
                    "cannot tell the file type of {s:?}: expected a csv, tsv, json, jsonl, \
                     ndjson, parquet, arrow, avro or xlsx extension{hint}"
                )));
            }
        };

        Ok(Self::File {
            // `~` is meaningless in a bucket key.
            path: match store {
                None => shellexpand::tilde(s).into_owned(),
                Some(_) => s.to_string(),
            },
            store,
            format,
        })
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
}

fn parse_url(s: &str) -> Result<Url, Error> {
    Url::parse(s).map_err(|e| Error::InvalidArgument(format!("bad source uri {s:?}: {e}")))
}
