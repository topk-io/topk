use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::sync::OnceLock;
use std::thread::{self, JoinHandle};

use duckdb::arrow::array::ArrayRef;
use duckdb::Connection;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::ReceiverStream;
use url::Url;

use crate::import::error::Error;
use crate::import::source::codec::arrow;
use crate::import::spec::{Field, Target};
use crate::import::value::id_string;
use crate::import::ID;

use super::{redact, Chunk, Record, Records, Table};

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
                    _ if s.contains("://") => {
                        ". To read from a database, pass it as the source, \
                         e.g. `topk import postgres://host/db --spec <spec>`"
                    }
                    _ => "",
                };
                return Err(Error::InvalidArgument(format!(
                    "cannot tell the file type of {s:?}: expected a csv, tsv, json, jsonl, \
                     ndjson, parquet, arrow, avro or xlsx extension{hint}"
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

/// A local, non-glob path — the only kind that can be stat'ed.
fn local_path(path: &str) -> Option<&str> {
    let path = path.trim_start_matches("file://");
    (!path.contains(['*', '?', '['])).then_some(path)
}

fn stem(path: &str) -> Option<String> {
    // A presigned/query-carrying http url names its file in the path alone.
    let path = match path.starts_with("http://") || path.starts_with("https://") {
        true => path.split(['?', '#']).next().unwrap_or(path),
        false => path,
    };
    let path = Path::new(local_path(path)?.trim_end_matches('/'));
    // data.csv.gz names the collection "data", not "data.csv".
    let path = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("gz") || ext.eq_ignore_ascii_case("zst") => {
            Path::new(path.file_stem()?)
        }
        _ => path,
    };
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
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
            std::env::temp_dir()
                .join("topk-import")
                .display()
                .to_string()
                .replace('\'', "''")
        ))?;
        let install = |ext: &str| {
            conn.execute_batch(&format!("INSTALL {ext}; LOAD {ext};"))
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
                    Format::Xlsx => install("excel")?,
                    // Community extension: a duckdb bump can 404 it.
                    Format::Arrow => conn
                        .execute_batch("INSTALL arrow FROM community; LOAD arrow;")
                        .map_err(|e| extension_error("arrow", e))?,
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
            dsn.replace('\'', "''"),
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
    ) -> Result<Records, Error> {
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
                    None => source.table(&source.connect(None)?, &target, after.as_deref(), &tx),
                }
            };
            if let Err(e) = produce() {
                let _ = tx.blocking_send(Err(e));
            }
        });
        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    /// An attached table ordered by the id column, so a batch's last id is a
    /// resume point. Postgres sorts on its own key via `postgres_query`;
    /// `mysql_query` breaks under a prepared statement ("Lost connection to
    /// server during query"), so mysql sorts in duckdb like sqlite.
    fn table(
        &self,
        conn: &Connection,
        target: &Target,
        after: Option<&str>,
        tx: &Sender,
    ) -> Result<(), Error> {
        read(conn, Select::table(self, target, after), tx).map(|_| ())
    }
}

type Sender = mpsc::Sender<Result<Chunk, Error>>;
type Receiver = mpsc::Receiver<Result<Chunk, Error>>;

fn select_sql(
    relation: &str,
    predicates: impl IntoIterator<Item = Option<String>>,
    order: Option<&str>,
    limit: Option<u64>,
    offset: u64,
) -> String {
    let mut sql = format!("SELECT * FROM {relation}");
    let mut has_where = false;
    for predicate in predicates.into_iter().flatten() {
        sql.push_str(if has_where { " AND " } else { " WHERE " });
        sql.push_str(&predicate);
        has_where = true;
    }
    if let Some(order) = order {
        sql.push_str(&format!(" ORDER BY {order}"));
    }
    if let Some(limit) = limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }
    if offset > 0 {
        sql.push_str(&format!(" OFFSET {offset}"));
    }
    sql
}

/// A planned DuckDB query. Construction owns SQL generation and resume
/// semantics; execution only needs a connection and somewhere to send chunks.
struct Select {
    sql: String,
    from: String,
    filter: Option<String>,
    position: Position,
}

impl Select {
    fn table(source: &Duckdb, target: &Target, after: Option<&str>) -> Select {
        let id = target.id.as_deref().unwrap_or(ID);
        let order = quoted(id, '"');
        let relation = match source {
            Duckdb::Postgres(_) => quoted(&target.from, '"'),
            _ => format!("src.{}", quoted(&target.from, '"')),
        };
        let sql = select_sql(
            &relation,
            [
                target.filter.as_ref().map(|filter| format!("({filter})")),
                after.map(|after| format!("{} > '{}'", order, after.replace('\'', "''"))),
            ],
            Some(&order),
            target.limit,
            0,
        );
        let sql = match source {
            Duckdb::Postgres(_) => format!(
                "SELECT * FROM postgres_query('src', '{}')",
                sql.replace('\'', "''")
            ),
            _ => sql,
        };
        Select {
            sql,
            from: target.from.clone(),
            filter: target.filter.clone(),
            position: Position::Id(id.to_string()),
        }
    }

    fn file(file: &File, target: &Target, offset: u64, limit: Option<u64>) -> Select {
        Select {
            sql: select_sql(
                &reader(file),
                [target.filter.as_ref().map(|filter| format!("({filter})"))],
                None,
                limit,
                offset,
            ),
            from: file.path.clone(),
            filter: target.filter.clone(),
            position: Position::Offset(offset),
        }
    }
}

enum Position {
    Id(String),
    Offset(u64),
}

impl Position {
    fn advance(&mut self, from: &str, rows: &[Result<Record, Error>]) -> Option<String> {
        match self {
            Position::Id(id) => {
                let record = rows.last()?.as_ref().ok()?;
                let (_, value) = record.iter().find(|(key, _)| key == id)?;
                id_string(id, value.clone()).ok()
            }
            Position::Offset(offset) => {
                *offset += rows.len() as u64;
                Some(format!("{from}:{offset}"))
            }
        }
    }
}

/// Files decoded ahead of the one being upserted. A single-row-group parquet is
/// read whole before its first row, so each file boundary stalls the sink (s3:
/// 29 → 41 MiB/s with one ahead; two measured the same for another row group
/// of memory). Each holds a `READER_MEMORY` connection, budgeted by `max_readers`.
const READ_AHEAD: usize = 1;

/// Files in glob order, one query each, read `READ_AHEAD` deep and forwarded in
/// order. The cursor is `<file>:<rows read>`: resuming skips files before it and
/// `OFFSET`s into the one it names.
fn files(
    source: &Duckdb,
    conn: &Connection,
    file: &File,
    target: &Target,
    after: Option<&str>,
    tx: &Sender,
) -> Result<(), Error> {
    let path = &file.path;
    let files: Vec<String> = match local_path(path).is_some() {
        true => vec![path.clone()],
        false => {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT file FROM glob('{}') ORDER BY 1",
                    path.replace('\'', "''")
                ))
                .map_err(|e| read_error(path, None, e))?;
            stmt.query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()
                .map_err(|e| read_error(path, None, e))?
        }
    };
    let after = after.and_then(|after| {
        let (file, offset) = after.rsplit_once(':')?;
        Some((file.to_string(), offset.parse::<u64>().ok()?))
    });
    // Globs list in byte order, which is how the cursor compares.
    let planned: Vec<(String, u64)> = files
        .into_iter()
        .filter_map(|file| match &after {
            Some((done, _)) if file < *done => None,
            Some((done, offset)) if file == *done => Some((file, *offset)),
            _ => Some((file, 0)),
        })
        .collect();
    // A limit is a running budget: the next query depends on the previous count.
    let ahead = match target.limit {
        Some(_) => 0,
        None => READ_AHEAD,
    };
    let mut remaining = target.limit;
    let mut planned = planned.into_iter();
    let mut readers: VecDeque<(Receiver, JoinHandle<Result<u64, Error>>)> = VecDeque::new();
    loop {
        while readers.len() < ahead + 1 && remaining != Some(0) {
            let Some((path, offset)) = planned.next() else {
                break;
            };
            let (file_tx, file_rx) = mpsc::channel(2);
            let source = source.clone();
            let target = target.clone();
            let file = File {
                path,
                ..file.clone()
            };
            let limit = remaining;
            readers.push_back((
                file_rx,
                thread::spawn(move || read_file(&source, &file, &target, offset, limit, &file_tx)),
            ));
        }
        let Some((mut rx, reader)) = readers.pop_front() else {
            break;
        };
        while let Some(chunk) = rx.blocking_recv() {
            if tx.blocking_send(chunk).is_err() {
                return Ok(());
            }
        }
        let read = reader
            .join()
            .map_err(|_| Error::InvalidArgument("file reader panicked".to_string()))??;
        if let Some(n) = remaining.as_mut() {
            *n = n.saturating_sub(read);
        }
    }
    Ok(())
}

/// One file on its own connection, from `offset`, at most `limit` rows.
fn read_file(
    source: &Duckdb,
    file: &File,
    target: &Target,
    offset: u64,
    limit: Option<u64>,
    tx: &Sender,
) -> Result<u64, Error> {
    let conn = source.connect(Some(file))?;
    read(&conn, Select::file(file, target, offset, limit), tx)
}

/// Executes one planned query. Returns the rows read and stops without error
/// if the receiver goes away.
fn read(conn: &Connection, mut select: Select, tx: &Sender) -> Result<u64, Error> {
    let mut stmt = conn
        .prepare(&select.sql)
        .map_err(|e| read_error(&select.from, select.filter.as_deref(), e))?;
    // `query_arrow` materializes the whole result; `stream_arrow` streams.
    // Stepped by hand: the iterator panics on a mid-stream failure, `step`
    // returns it.
    let _ = stmt
        .stream_arrow([])
        .map_err(|e| read_error(&select.from, select.filter.as_deref(), e))?;
    let mut read = 0;
    while let Some(array) = stmt
        .step()
        .map_err(|e| read_error(&select.from, select.filter.as_deref(), e))?
    {
        let batch = duckdb::arrow::record_batch::RecordBatch::from(&array);
        let schema = batch.schema();
        let columns: Vec<(&str, ArrayRef)> = schema
            .fields()
            .iter()
            .zip(batch.columns())
            .map(|(field, array)| (field.name().as_str(), array.clone()))
            .collect();
        let rows: Vec<Result<Record, Error>> = (0..batch.num_rows())
            .map(|row| {
                columns
                    .iter()
                    .map(|(name, array)| Ok((name.to_string(), arrow::value(array, row)?)))
                    .collect::<Result<Record, Error>>()
            })
            .collect();
        read += rows.len() as u64;
        let mark = select.position.advance(&select.from, &rows);
        if tx.blocking_send(Ok(Chunk { rows, mark })).is_err() {
            return Ok(read);
        }
    }
    Ok(read)
}

fn reader(file: &File) -> String {
    let reader = match file.format {
        Format::Csv => "read_csv_auto",
        Format::Json => "read_json_auto",
        Format::Parquet => "read_parquet",
        Format::Arrow => "read_arrow",
        Format::Avro => "read_avro",
        Format::Xlsx => "read_xlsx",
    };
    format!("{reader}('{}')", file.path.replace('\'', "''"))
}

/// `schema.table` as a quoted path, `"` for postgres/duckdb, `` ` `` for mysql.
fn quoted(name: &str, quote: char) -> String {
    let escaped = format!("{quote}{quote}");
    name.split('.')
        .map(|part| format!("{quote}{}{quote}", part.replace(quote, &escaped)))
        .collect::<Vec<_>>()
        .join(".")
}

/// Drops the `LINE n: …` block: the SQL is ours, not the user's.
fn strip_sql(e: &duckdb::Error) -> String {
    let msg = e.to_string();
    msg.split("\nLINE ")
        .next()
        .unwrap_or(&msg)
        .trim()
        .to_string()
}

fn extension_error(ext: &str, e: duckdb::Error) -> Error {
    Error::InvalidArgument(format!(
        "cannot load the duckdb {ext} extension (downloaded on first use — are you offline?): {}",
        strip_sql(&e)
    ))
}

/// A synthetic AWS profile whose `credential_process` is `aws configure
/// export-credentials`: the aws CLI resolves SSO → assume-role, which duckdb's
/// C++ chain cannot, and `REFRESH auto` re-runs it on expiry. None when env
/// keys are set, no `aws` is on PATH, or there is no profile. Sets
/// `AWS_CONFIG_FILE`, so `main` runs it before the runtime spawns threads.
pub fn aws_process_profile() -> Option<&'static str> {
    static PROFILE: OnceLock<Option<String>> = OnceLock::new();
    PROFILE
        .get_or_init(|| {
            if std::env::var("AWS_ACCESS_KEY_ID").is_ok() {
                return None;
            }
            let aws = std::env::split_paths(&std::env::var_os("PATH")?)
                .map(|dir| dir.join("aws"))
                .find(|p| p.is_file())?;
            let profile = std::env::var("AWS_PROFILE").unwrap_or_else(|_| "default".into());
            // The inner command must read the real config, not the synthetic one.
            let inner = match std::env::var("AWS_CONFIG_FILE") {
                Ok(original) => format!("AWS_CONFIG_FILE='{original}'"),
                Err(_) => {
                    // No profile anywhere: leave the default chain alone.
                    if !dirs::home_dir()?.join(".aws/config").is_file() {
                        return None;
                    }
                    "-u AWS_CONFIG_FILE".to_string()
                }
            };
            let path = std::env::temp_dir().join(format!("topk-aws-{}.ini", std::process::id()));
            std::fs::write(
                &path,
                format!(
                    "[profile topk-import]\ncredential_process = env {inner} '{}' \
                     configure export-credentials --profile '{profile}' --format process\n",
                    aws.display()
                ),
            )
            .ok()?;
            std::env::set_var("AWS_CONFIG_FILE", &path);
            Some("topk-import".to_string())
        })
        .as_deref()
}

fn secret(conn: &Connection, store: &ObjectStore) -> Result<(), Error> {
    // `REFRESH auto`: everything but a static key pair expires within the run.
    match store {
        ObjectStore::S3 => match std::env::var("AWS_ENDPOINT_URL") {
            Ok(endpoint) => {
                let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".into());
                let use_ssl = endpoint.starts_with("https");
                let host = endpoint
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
                    .replace('\'', "''");
                let region = region.replace('\'', "''");
                conn.execute_batch(&format!(
                    "CREATE OR REPLACE SECRET s3 (TYPE s3, PROVIDER credential_chain, \
                     REFRESH auto, CHAIN 'env', REGION '{region}', ENDPOINT '{host}', \
                     URL_STYLE 'path', USE_SSL {use_ssl});"
                ))?;
            }
            Err(_) => match aws_process_profile() {
                Some(profile) => conn
                    .execute_batch(&format!(
                        "CREATE OR REPLACE SECRET s3 (TYPE s3, PROVIDER credential_chain, \
                         CHAIN 'process', PROFILE '{profile}', REFRESH auto);"
                    ))
                    .map_err(|e| {
                        Error::InvalidArgument(format!(
                            "{} — if your AWS profile uses SSO, run `aws sso login`",
                            strip_sql(&e)
                        ))
                    })?,
                None => conn.execute_batch(
                    "CREATE OR REPLACE SECRET s3 (TYPE s3, PROVIDER credential_chain, REFRESH auto);",
                )?,
            },
        },
        ObjectStore::Gcs => {
            conn.execute_batch(
                "CREATE OR REPLACE SECRET gcs (TYPE gcs, PROVIDER credential_chain, REFRESH auto);",
            )?;
        }
        ObjectStore::HuggingFace => {
            conn.execute_batch(
                "CREATE OR REPLACE SECRET hf (TYPE huggingface, PROVIDER credential_chain);",
            )?;
        }
        ObjectStore::Azure => {
            conn.execute_batch("INSTALL azure; LOAD azure;")
                .map_err(|e| extension_error("azure", e))?;
            // The azure extension rejects REFRESH.
            conn.execute_batch(
                "CREATE OR REPLACE SECRET az (TYPE azure, PROVIDER credential_chain);",
            )?;
        }
        // httpfs alone; anonymous GET.
        ObjectStore::Http => {}
    }
    Ok(())
}

fn read_error(from: &str, filter: Option<&str>, e: duckdb::Error) -> Error {
    let msg = strip_sql(&e);
    Error::InvalidArgument(match filter {
        Some(filter) => format!("reading {from}: {msg} (filter: {filter:?})"),
        None => format!("reading {from}: {msg}"),
    })
}
