use std::path::Path;
use std::str::FromStr;

use crate::import::error::Error;

use super::lit;

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
}

/// A file or glob, local or in a bucket, typed by its extension.
#[derive(Debug, Clone)]
pub struct File {
    pub path: String,
    pub store: Option<ObjectStore>,
    pub format: Format,
}

impl FromStr for File {
    type Err = Error;

    fn from_str(s: &str) -> Result<File, Error> {
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

        let format = match extension(Path::new(logical(s))).as_deref() {
            Some("csv" | "tsv") => Format::Csv,
            Some("json" | "jsonl" | "ndjson") => Format::Json,
            Some("parquet") => Format::Parquet,
            Some("arrow") => Format::Arrow,
            Some("avro") => Format::Avro,
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
                    _ if s.contains("://") => {
                        ". To read from a database, pass it as the source, \
                         e.g. `topk import postgres://host/db --spec <spec>`"
                    }
                    _ => "",
                };
                return Err(Error::InvalidArgument(format!(
                    "cannot tell the file type of {s:?}: expected a csv, tsv, json, jsonl, \
                     ndjson, parquet, arrow or avro extension{hint}"
                )));
            }
        };

        Ok(File {
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

/// The part of a path that names the file: without an http query string (only
/// http, since `?` is a glob character elsewhere), and without a compression
/// suffix duckdb decodes transparently — data.csv.gz is a csv named data.
fn logical(path: &str) -> &str {
    let path = match path.starts_with("http://") || path.starts_with("https://") {
        true => path.split(['?', '#']).next().unwrap_or(path),
        false => path,
    };
    match extension(Path::new(path)).as_deref() {
        Some("gz" | "zst") => Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(path),
        _ => path,
    }
}

/// A local, non-glob path — the only kind that can be stat'ed.
pub(super) fn local_path(path: &str) -> Option<&str> {
    let path = path.trim_start_matches("file://");
    (!path.contains(['*', '?', '['])).then_some(path)
}

pub(super) fn stem(path: &str) -> Option<String> {
    Path::new(local_path(logical(path))?.trim_end_matches('/'))
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
}

/// Reads a fixed list of files instead of the locator: the catalog only needs a
/// representative schema, and binding a glob's union means a footer read per
/// file — minutes before the first row on a large object store.
pub(super) fn reader_of(file: &File, paths: &[String]) -> String {
    let list: Vec<String> = paths.iter().map(|p| format!("'{}'", lit(p))).collect();
    let (reader, opts) = reader_parts(file);
    format!("{reader}([{}]{opts})", list.join(", "))
}

pub(super) fn reader(file: &File) -> String {
    let (reader, opts) = reader_parts(file);
    format!("{reader}('{}'{opts})", lit(&file.path))
}

/// Take the union of every file a glob matches, so a column present in only
/// some files is kept, not dropped to the first file's schema.
fn reader_parts(file: &File) -> (&'static str, &'static str) {
    match file.format {
        Format::Csv => ("read_csv_auto", ", union_by_name=true"),
        Format::Json => ("read_json_auto", ", union_by_name=true"),
        Format::Parquet => ("read_parquet", ", union_by_name=true"),
        Format::Arrow => ("read_arrow", ""),
        Format::Avro => ("read_avro", ""),
    }
}
