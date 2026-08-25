use std::path::Path;

use duckdb::arrow::array::ArrayRef;
use duckdb::Connection;
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::ReceiverStream;

use crate::import::error::Error;
use crate::import::source::codec::{arrow, id_string};
use crate::import::source::uri::{Format, ObjectStore, Uri};
use crate::import::spec::{Field, Target};
use crate::import::ID;

use super::{Chunk, Record, Records, Table};

#[derive(Clone)]
pub enum Duckdb {
    Postgres(String),
    Mysql(String),
    Sqlite(String),
    /// Path-free: each `target.from` is its own locator.
    Files,
}

/// Per-reader duckdb budget. duckdb's default is 80% of RAM *per connection*,
/// which concurrent readers multiply into an OOM-kill; a 1.3 GiB single-row-group
/// parquet needs the whole column chunk resident.
const READER_MEMORY: &str = "4GiB";

/// How many `READER_MEMORY` readers fit in RAM. duckdb's default `memory_limit`
/// is 80% of RAM (cgroup-aware), which is how RAM is read without a dependency.
pub fn max_readers() -> Option<usize> {
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
    /// `from` as a file locator.
    pub fn file(from: &str) -> Result<(String, Option<ObjectStore>, Format), Error> {
        match from.parse() {
            Ok(Uri::File {
                path,
                store,
                format,
            }) => Ok((path, store, format)),
            _ => Err(Error::InvalidArgument(format!(
                "{from:?} is not a file path — pass the source to read it from, \
                 e.g. `topk import postgres://host/db --spec <spec>`"
            ))),
        }
    }

    /// A connection able to read `from`; attached sources ignore it and use their DSN.
    fn connect(&self, from: &str) -> Result<Connection, Error> {
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
            Duckdb::Postgres(url) => ("postgres", url),
            Duckdb::Mysql(url) => ("mysql", url),
            Duckdb::Sqlite(path) => ("sqlite", path),
            Duckdb::Files => {
                let (path, store, format) = Self::file(from)?;
                match format {
                    Format::Avro => install("avro")?,
                    Format::Xlsx => install("excel")?,
                    // Community extension: a duckdb bump can 404 it.
                    Format::Arrow => conn
                        .execute_batch("INSTALL arrow FROM community; LOAD arrow;")
                        .map_err(|e| extension_error("arrow", e))?,
                    Format::Csv | Format::Json | Format::Parquet => {}
                }
                if let Some(store) = store {
                    // Without an endpoint, duckdb resolves r2:// against
                    // amazonaws.com — silently querying the wrong cloud.
                    if path.starts_with("r2://") && std::env::var("AWS_ENDPOINT_URL").is_err() {
                        return Err(Error::InvalidArgument(
                            "r2:// needs AWS_ENDPOINT_URL=https://<account-id>.r2.cloudflarestorage.com \
                             and R2 keys in AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY"
                                .to_string(),
                        ));
                    }
                    install("httpfs")?;
                    secret(&conn, &store)?;
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
        .map_err(|e| Error::InvalidArgument(strip_sql(&e).replace(dsn.as_str(), &redact(dsn))))?;
        Ok(conn)
    }

    fn table_ref(&self, from: &str) -> Result<String, Error> {
        Ok(match self {
            Duckdb::Files => {
                let (path, _, format) = Self::file(from)?;
                reader(format, &path)
            }
            _ => format!("src.{}", quoted(from, '"')),
        })
    }

    fn list(&self, conn: &Connection) -> Result<Vec<(String, Option<String>)>, Error> {
        let hidden = match self {
            Duckdb::Postgres(_) => "'information_schema', 'pg_catalog'",
            Duckdb::Mysql(_) => "'information_schema', 'mysql', 'sys', 'performance_schema'",
            Duckdb::Sqlite(_) | Duckdb::Files => "''",
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

    /// The single-column primary key of `from`; None for composite keys.
    fn primary_key(&self, conn: &Connection, from: &str) -> Result<Option<String>, Error> {
        // duckdb_constraints() materializes every attached table's constraints
        // (seconds on a real database) and sees no mysql PKs at all; ask the
        // source's information_schema. sqlite has none but is small.
        let sql = match self {
            Duckdb::Files => return Ok(None),
            Duckdb::Sqlite(_) => {
                "SELECT constraint_column_names[1] FROM duckdb_constraints() \
                 WHERE constraint_type = 'PRIMARY KEY' AND database_name = 'src' \
                   AND schema_name || '.' || table_name = ? \
                   AND len(constraint_column_names) = 1"
            }
            Duckdb::Postgres(_) | Duckdb::Mysql(_) => {
                "SELECT kcu.column_name FROM src.information_schema.table_constraints tc \
                 JOIN src.information_schema.key_column_usage kcu \
                   ON tc.constraint_name = kcu.constraint_name \
                  AND tc.table_schema = kcu.table_schema \
                  AND tc.table_name = kcu.table_name \
                 WHERE tc.constraint_type = 'PRIMARY KEY' \
                   AND tc.table_schema || '.' || tc.table_name = ?"
            }
        };
        let mut stmt = conn.prepare(sql)?;
        let keys: Vec<String> = stmt
            .query_map(duckdb::params![from], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(match keys.as_slice() {
            [key] => Some(key.clone()),
            _ => None,
        })
    }

    pub async fn catalog(&self, uri: &Uri) -> Result<Vec<Table>, Error> {
        let source = self.clone();
        let uri = uri.clone();
        spawn_blocking(move || -> Result<Vec<Table>, Error> {
            let conn = source.connect(match &uri {
                Uri::File { path, .. } => path,
                _ => "",
            })?;
            // An empty file reads as a synthetic `column0`.
            if let Uri::File {
                path, store: None, ..
            } = &uri
            {
                if local_path(path)
                    .and_then(|p| std::fs::metadata(p).ok())
                    .is_some_and(|meta| meta.len() == 0)
                {
                    return Err(Error::InvalidArgument(format!(
                        "{path} is empty — nothing to import"
                    )));
                }
            }
            let objects = match &uri {
                // A file source's one object is the uri itself.
                Uri::File { path, .. } => vec![(path.clone(), stem(path))],
                _ => source.list(&conn)?,
            };
            let mut tables = Vec::with_capacity(objects.len());
            for (from, collection_hint) in objects {
                let table_ref = source.table_ref(&from)?;
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
                let primary_key = source.primary_key(&conn, &from)?;
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

    pub async fn stream(&self, target: &Target, after: Option<&str>) -> Result<Records, Error> {
        let source = self.clone();
        let target = target.clone();
        let after = after.map(str::to_string);
        // One message per arrow batch; per-row sends make the channel the bottleneck.
        let (tx, rx) = tokio::sync::mpsc::channel::<Chunk>(2);
        spawn_blocking(move || {
            let produce = || -> Result<(), Error> {
                let conn = source.connect(&target.from)?;
                match source {
                    Duckdb::Files => files(&source, &conn, &target, after.as_deref(), &tx),
                    _ => source.table(&conn, &target, after.as_deref(), &tx),
                }
            };
            if let Err(e) = produce() {
                let _ = tx.blocking_send(Chunk {
                    rows: vec![Err(e)],
                    mark: None,
                });
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
        let id = target.id.as_deref().unwrap_or(ID);
        let mut clauses: Vec<String> = Vec::new();
        if let Some(filter) = target.filter.as_deref() {
            clauses.push(format!("({filter})"));
        }
        let (quote, passthrough) = match self {
            Duckdb::Postgres(_) => ('"', Some("postgres_query")),
            _ => ('"', None),
        };
        // Quoted: the source coerces it to the column's type and keeps the index.
        if let Some(after) = after {
            clauses.push(format!(
                "{} > '{}'",
                quoted(id, quote),
                after.replace('\'', "''")
            ));
        }
        let where_ = match clauses.is_empty() {
            true => String::new(),
            false => format!(" WHERE {}", clauses.join(" AND ")),
        };
        let limit = target
            .limit
            .map(|n| format!(" LIMIT {n}"))
            .unwrap_or_default();
        let sql = match passthrough {
            Some(function) => format!(
                "SELECT * FROM {function}('src', '{}')",
                format!(
                    "SELECT * FROM {}{where_} ORDER BY {}{limit}",
                    quoted(&target.from, quote),
                    quoted(id, quote)
                )
                .replace('\'', "''")
            ),
            None => format!(
                "SELECT * FROM {}{where_} ORDER BY {}{limit}",
                self.table_ref(&target.from)?,
                quoted(id, '"')
            ),
        };
        let mark = |rows: &[Result<Record, Error>]| match rows.last()? {
            Ok(record) => {
                let (_, value) = record.iter().find(|(key, _)| key == id)?;
                id_string(id, value.clone()).ok()
            }
            Err(_) => None,
        };
        run(conn, &sql, &target.from, target.filter.as_deref(), tx, mark)?;
        Ok(())
    }
}

type Sender = tokio::sync::mpsc::Sender<Chunk>;

/// Files decoded ahead of the one being upserted. A single-row-group parquet is
/// read whole before its first row, so each file boundary stalls the sink (s3:
/// 29 → 41 MiB/s with one ahead; two measured the same for another row group
/// of memory). Each holds a `READER_MEMORY` connection, budgeted by `max_readers`.
pub const READ_AHEAD: usize = 1;

/// Files in glob order, one query each, read `READ_AHEAD` deep and forwarded in
/// order. The cursor is `<file>:<rows read>`: resuming skips files before it and
/// `OFFSET`s into the one it names.
fn files(
    source: &Duckdb,
    conn: &Connection,
    target: &Target,
    after: Option<&str>,
    tx: &Sender,
) -> Result<(), Error> {
    let (path, _, _) = Duckdb::file(&target.from)?;
    let files: Vec<String> = match local_path(&path).is_some() {
        true => vec![path],
        false => {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT file FROM glob('{}') ORDER BY 1",
                    path.replace('\'', "''")
                ))
                .map_err(|e| read_error(&path, None, e))?;
            stmt.query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()
                .map_err(|e| read_error(&path, None, e))?
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
    let mut readers: std::collections::VecDeque<(
        tokio::sync::mpsc::Receiver<Chunk>,
        std::thread::JoinHandle<Result<u64, Error>>,
    )> = std::collections::VecDeque::new();
    loop {
        while readers.len() < ahead + 1 && remaining != Some(0) {
            let Some((file, offset)) = planned.next() else {
                break;
            };
            let (file_tx, file_rx) = tokio::sync::mpsc::channel::<Chunk>(2);
            let source = source.clone();
            let target = target.clone();
            let limit = remaining;
            readers.push_back((
                file_rx,
                std::thread::spawn(move || {
                    read_file(&source, &target, &file, offset, limit, &file_tx)
                }),
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
    target: &Target,
    file: &str,
    offset: u64,
    limit: Option<u64>,
    tx: &Sender,
) -> Result<u64, Error> {
    let (_, _, format) = Duckdb::file(&target.from)?;
    let conn = source.connect(&target.from)?;
    let mut sql = format!("SELECT * FROM {}", reader(format, file));
    if let Some(filter) = target.filter.as_deref() {
        sql.push_str(&format!(" WHERE ({filter})"));
    }
    if let Some(n) = limit {
        sql.push_str(&format!(" LIMIT {n}"));
    }
    if offset > 0 {
        sql.push_str(&format!(" OFFSET {offset}"));
    }
    let mut position = offset;
    run(&conn, &sql, file, target.filter.as_deref(), tx, |rows| {
        position += rows.len() as u64;
        Some(format!("{file}:{position}"))
    })
}

/// Streams `sql` to the sink in arrow batches, marking each with `mark`.
/// Returns the rows read; stops early, without error, once the sink is gone.
fn run(
    conn: &Connection,
    sql: &str,
    from: &str,
    filter: Option<&str>,
    tx: &Sender,
    mut mark: impl FnMut(&[Result<Record, Error>]) -> Option<String>,
) -> Result<u64, Error> {
    let mut stmt = conn.prepare(sql).map_err(|e| read_error(from, filter, e))?;
    // `query_arrow` materializes the whole result; `stream_arrow` streams.
    // Stepped by hand: the iterator panics on a mid-stream failure, `step`
    // returns it.
    let _ = stmt
        .stream_arrow([])
        .map_err(|e| read_error(from, filter, e))?;
    let mut read = 0;
    while let Some(array) = stmt.step().map_err(|e| read_error(from, filter, e))? {
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
                    .map_err(|e| Error::Row(Box::new(e)))
            })
            .collect();
        read += rows.len() as u64;
        let mark = mark(&rows);
        if tx.blocking_send(Chunk { rows, mark }).is_err() {
            return Ok(read);
        }
    }
    Ok(read)
}

fn reader(format: Format, path: &str) -> String {
    let reader = match format {
        Format::Csv => "read_csv_auto",
        Format::Json => "read_json_auto",
        Format::Parquet => "read_parquet",
        Format::Arrow => "read_arrow",
        Format::Avro => "read_avro",
        Format::Xlsx => "read_xlsx",
    };
    format!("{reader}('{}')", path.replace('\'', "''"))
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
/// keys are set, no `aws` is on PATH, or there is no profile.
fn aws_process_profile() -> Option<&'static str> {
    static PROFILE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
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

fn redact(dsn: &str) -> String {
    match url::Url::parse(dsn) {
        Ok(mut url) if url.password().is_some() => {
            let _ = url.set_password(Some("***"));
            url.to_string()
        }
        _ => dsn.to_string(),
    }
}
