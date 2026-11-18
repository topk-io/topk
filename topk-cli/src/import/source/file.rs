use std::path::{is_separator, Path, PathBuf, MAIN_SEPARATOR};
use std::str::FromStr;

use crate::import::error::Error;

use super::SourceLocation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectStore {
    S3,
    Gcs,
    Azure,
    HuggingFace,
    /// Plain http(s) — read through httpfs, no credentials.
    Http,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Csv,
    Json,
    Parquet,
    Arrow,
    Avro,
}

/// A file or glob, local or in a bucket, typed by its extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub path: String,
    pub store: Option<ObjectStore>,
    pub format: Format,
    pub path_columns: Vec<(String, Segment)>,
}

impl File {
    pub fn glob(&self) -> String {
        self.path
            .split('/')
            .map(|segment| match column_name(segment) {
                Some(_) => "*",
                None => segment,
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    pub fn path_values<'a>(&'a self, path: &'a str) -> Result<Vec<(&'a str, &'a str)>, Error> {
        self.path_columns
            .iter()
            .map(|(name, segment)| {
                let value = match segment {
                    Segment::FromStart(n) => path.split(is_separator).nth(*n),
                    Segment::FromEnd(n) => path.rsplit(is_separator).nth(*n),
                };
                value.map(|value| (name.as_str(), value)).ok_or_else(|| {
                    Error::InvalidArgument(format!("{path:?} has no segment for {{{name}}}"))
                })
            })
            .collect()
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.store.is_some() {
            return Ok(());
        }
        let glob = self.glob();
        let path = match local_path(&glob) {
            Some(path) => path,
            None => return Ok(()),
        };
        let meta = std::fs::metadata(path).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => Error::InvalidArgument(format!(
                "file not found: {}",
                SourceLocation::Local(path.into())
            )),
            _ => error.into(),
        })?;
        // An empty file reads as a synthetic `column0`.
        if meta.len() == 0 {
            return Err(Error::InvalidArgument(format!(
                "{} is empty — nothing to import",
                SourceLocation::from(self)
            )));
        }
        Ok(())
    }
}

impl FromStr for File {
    type Err = Error;

    fn from_str(s: &str) -> Result<File, Error> {
        if matches!(s, "-" | "/dev/stdin" | "/dev/fd/0") {
            return Err(Error::InvalidArgument(
                "import does not read stdin: save the input to a file with an extension \
                 (e.g. data.jsonl) and import that file"
                    .into(),
            ));
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

        let format = match extension(Path::new(logical(s))).as_deref() {
            Some("csv" | "tsv") => Format::Csv,
            Some("json" | "jsonl" | "ndjson") => Format::Json,
            Some("parquet") => Format::Parquet,
            Some("arrow") => Format::Arrow,
            Some("avro") => Format::Avro,
            other => {
                let dir = s.trim_end_matches('/');
                let hint = match store {
                    Some(ObjectStore::Http) => {
                        let es = s
                            .trim_end_matches('/')
                            .replacen("https://", "es+https://", 1)
                            .replacen("http://", "es://", 1);
                        match es.matches('/').count() {
                            2 => format!(". For an Elasticsearch index, use {es}/<index>"),
                            _ => format!(". For an Elasticsearch index, use {es}"),
                        }
                    }
                    // A dataset repo is a tree of files, not a file; name one.
                    Some(ObjectStore::HuggingFace) => {
                        ". A hugging face dataset holds <config>/<split>-*.parquet files, \
                         or read hugging face's converted copy at \
                         hf://datasets/<org>/<name>@~parquet/<config>/<split>/*.parquet"
                            .into()
                    }
                    _ if matches!(other, Some("db" | "sqlite" | "sqlite3")) => {
                        format!(". For SQLite, use sqlite:{s}#<table>")
                    }
                    Some(ObjectStore::S3 | ObjectStore::Gcs | ObjectStore::Azure)
                        if other.is_none() =>
                    {
                        format!(". A source is one file or one glob: for the parquet files in this folder, use '{dir}/*.parquet', or '{dir}/**/*.parquet' to include subfolders")
                    }
                    None if Path::new(&*shellexpand::tilde(s)).is_dir() => {
                        let mut found = Vec::new();
                        for entry in std::fs::read_dir(&*shellexpand::tilde(s))? {
                            if let Some(ext) = extension(&entry?.path()) {
                                if matches!(
                                    ext.as_str(),
                                    "csv"
                                        | "tsv"
                                        | "json"
                                        | "jsonl"
                                        | "ndjson"
                                        | "parquet"
                                        | "arrow"
                                        | "avro"
                                ) {
                                    found.push(ext);
                                }
                            }
                        }
                        found.sort();
                        found.dedup();
                        let globs = match found.as_slice() {
                            [] => format!("'{dir}/**/*.parquet'"),
                            found => found
                                .iter()
                                .map(|ext| format!("'{dir}/*.{ext}'"))
                                .collect::<Vec<_>>()
                                .join(" or "),
                        };
                        format!(". A source is one file or one glob: for the files in this folder, use {globs}; \
                         '{dir}/{{name}}/*.parquet' also makes each sub-folder's name a column")
                    }
                    _ if s.contains("://") => ". To read from a database, pass it as the source, \
                         e.g. `topk import postgres://host/db#books --to books`"
                        .into(),
                    _ => String::new(),
                };
                return Err(Error::InvalidArgument(format!(
                    "cannot tell the file type of {s:?}: expected a csv, tsv, json, jsonl, \
                     ndjson, parquet, arrow or avro extension{hint}"
                )));
            }
        };

        let path = match store {
            // Absolute, so a saved run resumes from any directory.
            None => {
                // `[{]name}` matches a folder literally named `{name}` in the working or home directory.
                let literal = |dir: PathBuf| {
                    dir.display()
                        .to_string()
                        .replace(MAIN_SEPARATOR, "/")
                        .split('/')
                        .map(|segment| match column_name(segment) {
                            Some(_) => format!("[{{]{}", &segment[1..]),
                            None => segment.to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join("/")
                };
                let typed =
                    shellexpand::tilde_with_context(s.strip_prefix("file://").unwrap_or(s), || {
                        dirs::home_dir().map(literal)
                    });
                std::path::absolute(Path::new(&literal(std::env::current_dir()?)).join(&*typed))?
                    .display()
                    .to_string()
                    .replace(MAIN_SEPARATOR, "/")
            }
            // `~` is meaningless in a bucket key.
            Some(_) => s.to_string(),
        };
        let path_columns = path_columns(&path)?;
        if let (Some(ObjectStore::Http), Some((name, _))) = (&store, path_columns.first()) {
            return Err(Error::InvalidArgument(format!(
                "{{{name}}} in {path:?}: http(s) cannot list folders; read from a bucket or a local path"
            )));
        }
        Ok(File {
            path,
            store,
            format,
            path_columns,
        })
    }
}

/// Where a `{name}` column's value sits in a matched file's path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    FromStart(usize),
    FromEnd(usize),
}

fn column_name(segment: &str) -> Option<&str> {
    segment
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .filter(|name| {
            let mut ident = name.chars();
            ident
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && ident.all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

fn path_columns(path: &str) -> Result<Vec<(String, Segment)>, Error> {
    let segments: Vec<&str> = path.split('/').collect();
    let mut columns: Vec<(String, Segment)> = Vec::new();
    for (i, segment) in segments.iter().enumerate() {
        let name = match column_name(segment) {
            Some(name) => name,
            None => continue,
        };
        if columns.iter().any(|(column, _)| column == name) {
            return Err(Error::InvalidArgument(format!(
                "{{{name}}} appears twice in {path:?}"
            )));
        }
        let at = match (
            segments[..i].iter().any(|s| s.contains("**")),
            segments[i + 1..].iter().any(|s| s.contains("**")),
        ) {
            (_, false) => Segment::FromEnd(segments.len() - 1 - i),
            (false, true) => Segment::FromStart(i),
            (true, true) => {
                return Err(Error::InvalidArgument(format!(
                    "{{{name}}} in {path:?} has `**` on both sides, so its segment is not fixed: \
                     drop one `**`"
                )))
            }
        };
        columns.push((name.to_string(), at));
    }
    Ok(columns)
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
    (!path.contains(['*', '?', '['])).then_some(path)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::File;

    #[rstest]
    #[case::plain("/d/a.csv", "/d/a.csv", &[])]
    #[case::from_the_end("/d/{lang}/*.csv", "/d/sw/a.csv", &[("lang", "sw")])]
    #[case::bucket("s3://b/**/{lang}/x.csv", "s3://b/p/q/sw/x.csv", &[("lang", "sw")])]
    #[case::double_star_after("/{year}/**/x.csv", "/2024/a/b/x.csv", &[("year", "2024")])]
    #[case::double_star_empty("/{year}/**/x.csv", "/2024/x.csv", &[("year", "2024")])]
    #[case::file_scheme("file:///d/{year}/**/x.csv", "/d/2024/a/x.csv", &[("year", "2024")])]
    #[case::two_columns("/{year}/{lang}/x.csv", "/2024/sw/x.csv", &[("year", "2024"), ("lang", "sw")])]
    #[case::not_an_identifier("/d/{1lang}/x.csv", "/d/{1lang}/x.csv", &[])]
    #[case::part_of_a_segment("/d/lang={lang}/x.csv", "/d/lang={lang}/x.csv", &[])]
    #[case::braced_file_name("/d/{lang}.csv", "/d/{lang}.csv", &[])]
    fn values(#[case] pattern: &str, #[case] matched: &str, #[case] expected: &[(&str, &str)]) {
        let file: File = pattern.parse().unwrap();
        assert_eq!(file.path_values(matched).unwrap(), expected);
    }

    #[rstest]
    #[case::twice("{lang}/{lang}/x.csv", "appears twice")]
    #[case::double_star_both_sides("**/{lang}/**/x.csv", "`**` on both sides")]
    #[case::http("https://h/{lang}/x.csv", "http(s) cannot list folders")]
    fn refuses(#[case] path: &str, #[case] error: &str) {
        let err = path.parse::<File>().unwrap_err().to_string();
        assert!(err.contains(error), "{err}");
    }
}
