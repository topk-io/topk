mod creds;
mod file;
mod read;
mod select;

use std::collections::HashMap;
use std::fmt;

use duckdb::Connection;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::ReceiverStream;
use url::Url;

use crate::import::error::Error;
use crate::import::source::codec::arrow;
use crate::import::spec::{Field, Target};

use super::{redact, Chunk, ChunkStream, Table};

pub use creds::aws_process_profile;
use creds::secret;
pub use file::File;
use file::{local_path, reader, stem, Format};
use read::{files, table, READ_AHEAD};

#[derive(Clone)]
pub enum Duckdb {
    Postgres(Url),
    Mysql(Url),
    Sqlite(String),
    /// `Some` when a uri named one file or glob; `None` when each `target.from`
    /// is its own locator.
    Files(Option<File>),
}

impl fmt::Display for Duckdb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Duckdb::Postgres(url) | Duckdb::Mysql(url) => f.write_str(&redact(url)),
            Duckdb::Sqlite(path) => write!(f, "sqlite:{path}"),
            Duckdb::Files(Some(file)) => f.write_str(&file.path),
            Duckdb::Files(None) => Ok(()),
        }
    }
}

impl fmt::Debug for Duckdb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Duckdb::Postgres(url) => f.debug_tuple("Postgres").field(&redact(url)).finish(),
            Duckdb::Mysql(url) => f.debug_tuple("Mysql").field(&redact(url)).finish(),
            Duckdb::Sqlite(path) => f.debug_tuple("Sqlite").field(path).finish(),
            Duckdb::Files(file) => f.debug_tuple("Files").field(file).finish(),
        }
    }
}

/// A single-quoted SQL literal's body.
pub(super) fn lit(value: &str) -> String {
    value.replace('\'', "''")
}

/// Per-reader duckdb budget. duckdb's default is 80% of RAM *per connection*,
/// which concurrent readers multiply into an OOM-kill; a 1.3 GiB single-row-group
/// parquet needs the whole column chunk resident.
const READER_MEMORY: &str = "4GiB";

/// How many `READER_MEMORY` readers fit in RAM. duckdb's default `memory_limit`
/// is 80% of RAM (cgroup-aware), which is how RAM is read without a dependency.
fn max_readers() -> Option<usize> {
    let conn = Connection::open_in_memory().ok()?;
    let mut stmt = conn
        .prepare("SELECT value FROM duckdb_settings() WHERE name = 'memory_limit'")
        .ok()?;
    let reported: String = stmt.query_row([], |row| row.get(0)).ok()?;
    let reported = reported.parse::<bytesize::ByteSize>().ok()?.as_u64();
    let each = READER_MEMORY.parse::<bytesize::ByteSize>().ok()?.as_u64();
    // Two thirds of RAM: batches, arrow copies and proto encoding live outside
    // duckdb's accounting. 10/8 × 2/3 = 5/6.
    Some((reported * 5 / 6 / each).max(1) as usize)
}

impl Duckdb {
    /// Each duckdb scan holds a row group; too many OOM-kill the process
    /// regardless of the per-connection `memory_limit`.
    pub fn concurrency_limit(&self) -> usize {
        max_readers().map_or(usize::MAX, |readers| match self {
            Duckdb::Files(_) => (readers / (1 + READ_AHEAD)).max(1),
            _ => readers,
        })
    }

    /// A connection able to read `file`; attached sources ignore it and use their DSN.
    fn connect(&self, file: Option<&File>) -> Result<Connection, Error> {
        let conn = Connection::open_in_memory()?;
        // One thread, in order: rows arrive as stored, which makes a row offset
        // a resume point; more threads measured the same.
        conn.execute_batch(&format!(
            "SET preserve_insertion_order = true; SET memory_limit = '{READER_MEMORY}'; \
             SET threads = 1; SET temp_directory = '{}';",
            lit(&std::env::temp_dir()
                .join("topk-import")
                .display()
                .to_string())
        ))?;
        let install = |ext: &str| {
            conn.execute_batch(&format!("INSTALL {ext}; LOAD {ext};"))
                .map_err(|e| extension_error(ext, e))
        };
        // Community extensions live in a separate repo; a duckdb bump can 404 one.
        let install_community = |ext: &str| {
            conn.execute_batch(&format!("INSTALL {ext} FROM community; LOAD {ext};"))
                .map_err(|e| extension_error(ext, e))
        };

        let (ext, dsn) = match self {
            Duckdb::Postgres(url) => ("postgres", url.as_str()),
            Duckdb::Mysql(url) => ("mysql", url.as_str()),
            Duckdb::Sqlite(path) => ("sqlite", path.as_str()),
            Duckdb::Files(_) => {
                let file = file.ok_or_else(|| {
                    Error::InvalidArgument("no source: name a file to read".to_string())
                })?;
                match file.format {
                    Format::Avro => install("avro")?,
                    Format::Arrow => install_community("arrow")?,
                    Format::Csv | Format::Json | Format::Parquet => {}
                }
                if let Some(store) = &file.store {
                    // Without an endpoint, duckdb resolves r2:// against
                    // amazonaws.com — silently querying the wrong cloud.
                    if file.path.starts_with("r2://") && std::env::var("AWS_ENDPOINT_URL").is_err()
                    {
                        return Err(Error::InvalidArgument(
                            "r2:// needs AWS_ENDPOINT_URL=https://<account-id>.r2.cloudflarestorage.com \
                             and R2 keys in AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY"
                                .to_string(),
                        ));
                    }
                    install("httpfs")?;
                    secret(&conn, store)?;
                }
                return Ok(conn);
            }
        };
        install(ext)?;
        conn.execute_batch(&format!(
            "ATTACH '{}' AS src (TYPE {ext}, READ_ONLY);",
            lit(dsn),
        ))
        // Duckdb echoes the DSN verbatim, password included.
        .map_err(|e| {
            let redacted = match self {
                Duckdb::Postgres(url) | Duckdb::Mysql(url) => redact(url),
                _ => dsn.to_string(),
            };
            Error::InvalidArgument(strip_sql(&e).replace(dsn, &redacted))
        })?;
        Ok(conn)
    }

    fn list(&self, conn: &Connection) -> Result<Vec<(String, Option<String>)>, Error> {
        let hidden = match self {
            Duckdb::Postgres(_) => "'information_schema', 'pg_catalog'",
            Duckdb::Mysql(_) => "'information_schema', 'mysql', 'sys', 'performance_schema'",
            Duckdb::Sqlite(_) | Duckdb::Files(_) => "''",
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT schema_name || '.' || table_name, table_name FROM duckdb_tables() \
             WHERE database_name = 'src' AND schema_name NOT IN ({hidden}) \
             ORDER BY 1",
        ))?;
        let names = stmt
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, Some(r.get::<_, String>(1)?)))
            })?
            .collect::<Result<Vec<(String, Option<String>)>, _>>()?;
        Ok(names)
    }

    /// Single-column primary keys by qualified name, in one query: asked per
    /// table it cost 5ms each, for every table in the database.
    fn primary_keys(&self, conn: &Connection) -> Result<HashMap<String, String>, Error> {
        // duckdb_constraints() materializes every attached table's constraints
        // (seconds on a real database) and sees no mysql PKs at all; ask the
        // source's information_schema. sqlite has none but is small.
        let sql = match self {
            Duckdb::Files(_) => return Ok(HashMap::new()),
            Duckdb::Sqlite(_) => {
                "SELECT schema_name || '.' || table_name, constraint_column_names[1] \
                 FROM duckdb_constraints() \
                 WHERE constraint_type = 'PRIMARY KEY' AND database_name = 'src' \
                   AND len(constraint_column_names) = 1"
            }
            Duckdb::Postgres(_) | Duckdb::Mysql(_) => {
                "SELECT tc.table_schema || '.' || tc.table_name, min(kcu.column_name) \
                 FROM src.information_schema.table_constraints tc \
                 JOIN src.information_schema.key_column_usage kcu \
                   ON tc.constraint_name = kcu.constraint_name \
                  AND tc.table_schema = kcu.table_schema \
                  AND tc.table_name = kcu.table_name \
                 WHERE tc.constraint_type = 'PRIMARY KEY' \
                 GROUP BY 1 HAVING count(*) = 1"
            }
        };
        let mut stmt = conn.prepare(sql)?;
        Ok(stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?)
    }

    pub async fn catalog(&self) -> Result<Vec<Table>, Error> {
        let source = self.clone();
        spawn_blocking(move || -> Result<Vec<Table>, Error> {
            let file = match &source {
                Duckdb::Files(file) => file.clone(),
                _ => None,
            };
            let conn = source.connect(file.as_ref())?;
            // (object, collection hint, what to SELECT from)
            let objects: Vec<(String, Option<String>, String)> = match &file {
                // A file source's one object is the uri it was given.
                Some(file) => {
                    // An empty file reads as a synthetic `column0`.
                    if local_path(&file.path)
                        .and_then(|path| std::fs::metadata(path).ok())
                        .is_some_and(|meta| meta.len() == 0)
                    {
                        return Err(Error::InvalidArgument(format!(
                            "{} is empty — nothing to import",
                            file.path
                        )));
                    }
                    vec![(file.path.clone(), stem(&file.path), reader(file))]
                }
                None => source
                    .list(&conn)?
                    .into_iter()
                    .map(|(from, hint)| {
                        let table_ref = format!("src.{}", quoted(&from, '"'));
                        (from, hint, table_ref)
                    })
                    .collect(),
            };
            let mut keys = source.primary_keys(&conn)?;
            let mut tables = Vec::with_capacity(objects.len());
            for (from, collection_hint, table_ref) in objects {
                let mut stmt = conn
                    .prepare(&format!("SELECT * FROM {table_ref} LIMIT 0"))
                    .map_err(|e| read_error(&from, None, e))?;
                let schema = stmt
                    .query_arrow([])
                    .map_err(|e| read_error(&from, None, e))?
                    .get_schema();
                let columns: Vec<(String, Field)> = schema
                    .fields()
                    .iter()
                    .map(|field| {
                        (
                            field.name().clone(),
                            Field {
                                ty: arrow::ty(field.data_type()),
                                ..Default::default()
                            },
                        )
                    })
                    .collect();

                if columns.is_empty() {
                    return Err(Error::InvalidArgument(format!(
                        "no columns discovered for {table_ref}"
                    )));
                }
                let primary_key = keys.remove(&from);
                tables.push(Table {
                    from,
                    collection_hint,
                    columns,
                    primary_key,
                });
            }
            Ok(tables)
        })
        .await?
    }

    pub async fn stream(
        &self,
        file: Option<&File>,
        target: &Target,
        after: Option<&str>,
    ) -> Result<ChunkStream, Error> {
        let source = self.clone();
        let file = file.cloned();
        let target = target.clone();
        let after = after.map(str::to_string);
        // One message per arrow batch; per-row sends make the channel the bottleneck.
        let (tx, rx) = mpsc::channel::<Result<Chunk, Error>>(2);
        spawn_blocking(move || {
            let produce = || -> Result<(), Error> {
                match &file {
                    Some(file) => {
                        let conn = source.connect(Some(file))?;
                        files(&source, &conn, file, &target, after.as_deref(), &tx)
                    }
                    None => {
                        let conn = source.connect(None)?;
                        table(&source, &conn, &target, after.as_deref(), &tx)
                    }
                }
            };
            if let Err(e) = produce() {
                let _ = tx.blocking_send(Err(e));
            }
        });
        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}

/// `schema.table` as a quoted path, `"` for postgres/duckdb, `` ` `` for mysql.
pub(super) fn quoted(name: &str, quote: char) -> String {
    let escaped = format!("{quote}{quote}");
    name.split('.')
        .map(|part| format!("{quote}{}{quote}", part.replace(quote, &escaped)))
        .collect::<Vec<_>>()
        .join(".")
}

/// Drops the `LINE n: …` block: the SQL is ours, not the user's.
pub(super) fn strip_sql(e: &duckdb::Error) -> String {
    let msg = e.to_string();
    msg.split("\nLINE ")
        .next()
        .unwrap_or(&msg)
        .trim()
        .to_string()
}

pub(super) fn extension_error(ext: &str, e: duckdb::Error) -> Error {
    Error::InvalidArgument(format!(
        "cannot load the duckdb {ext} extension (downloaded on first use — are you offline?): {}",
        strip_sql(&e)
    ))
}

pub(super) fn read_error(from: &str, filter: Option<&str>, e: duckdb::Error) -> Error {
    let msg = strip_sql(&e);
    Error::InvalidArgument(match filter {
        Some(filter) => format!("reading {from}: {msg} (filter: {filter:?})"),
        None => format!("reading {from}: {msg}"),
    })
}
