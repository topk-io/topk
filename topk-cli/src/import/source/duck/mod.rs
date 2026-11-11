mod arrow;
mod creds;
mod read;
mod select;

use std::collections::HashMap;
use std::fmt;

use duckdb::Connection;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use url::Url;

use crate::import::error::Error;
use crate::import::spec::Target;

use super::{redact, ChunkStream, Cursor, Table};

use super::file::{local_path, stem};
use super::{File, Format};
pub use creds::aws_process_profile;
use creds::secret;
use read::{Plan, Sender, READ_AHEAD};
use select::Select;

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
const READER_MEMORY: u64 = 4 << 30;

/// Bytes all readers may hold together. duckdb's default `memory_limit` is 80% of
/// RAM (cgroup-aware), which is how RAM is read without a dependency.
fn reader_budget() -> Option<u64> {
    let conn = Connection::open_in_memory().ok()?;
    let mut stmt = conn
        .prepare("SELECT value FROM duckdb_settings() WHERE name = 'memory_limit'")
        .ok()?;
    let reported: String = stmt.query_row([], |row| row.get(0)).ok()?;
    let reported = reported.parse::<bytesize::ByteSize>().ok()?.as_u64();
    // Two thirds of RAM: batches, arrow copies and proto encoding live outside
    // duckdb's accounting. 10/8 × 2/3 = 5/6.
    Some(reported * 5 / 6)
}

/// A reader holds about one decoded row group: at most 8× a local file's size,
/// `READER_MEMORY` for anything that can't be stat'ed.
fn reader_memory(from: &str) -> u64 {
    local_path(from)
        .and_then(|path| std::fs::metadata(path).ok())
        .map_or(READER_MEMORY, |meta| (meta.len() * 8).min(READER_MEMORY))
}

impl Duckdb {
    /// Each duckdb scan holds a row group; too many OOM-kill the process
    /// regardless of the per-connection `memory_limit`.
    pub fn concurrency_limit<'a>(&self, targets: impl IntoIterator<Item = &'a Target>) -> usize {
        let budget = match reader_budget() {
            Some(budget) => budget,
            None => return usize::MAX,
        };
        let mut need: Vec<u64> = targets
            .into_iter()
            .map(|target| match self {
                Duckdb::Files(_) => reader_memory(&target.from) * (1 + READ_AHEAD as u64),
                _ => READER_MEMORY,
            })
            .collect();
        need.sort_unstable_by(|a, b| b.cmp(a));
        need.iter()
            .scan(0, |held, n| {
                *held += n;
                Some(*held)
            })
            .take_while(|held| *held <= budget)
            .count()
            .max(1)
    }

    /// A connection able to read `file`; attached sources ignore it and use their DSN.
    fn connect(&self, file: Option<&File>) -> Result<Connection, Error> {
        let conn = Connection::open_in_memory()?;
        // One thread, in order: rows arrive as stored, which makes a row offset
        // a resume point; more threads measured the same.
        conn.execute_batch(&format!(
            "SET preserve_insertion_order = true; SET memory_limit = '{}MiB'; \
             SET threads = 1; SET temp_directory = '{}';",
            READER_MEMORY >> 20,
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
            "ATTACH '{}' AS src (TYPE {ext}, READ_ONLY); USE src;",
            lit(dsn),
        ))
        // Duckdb echoes the DSN verbatim, password included.
        .map_err(|e| {
            let redacted = match self {
                Duckdb::Postgres(url) | Duckdb::Mysql(url) => redact(url),
                _ => dsn.to_string(),
            };
            Error::InvalidArgument(e.to_string().replace(dsn, &redacted))
        })?;
        Ok(conn)
    }

    /// Postgres runs the query itself via `postgres_query`; `mysql_query` breaks
    /// under a prepared statement ("Lost connection to server during query"),
    /// so mysql reads through duckdb like sqlite.
    fn select(&self, from: &str) -> Select {
        match self {
            Duckdb::Postgres(_) => Select::table(from).pushdown("src"),
            _ => Select::table(from),
        }
    }

    fn list(&self, conn: &Connection) -> Result<Vec<(String, Option<String>)>, Error> {
        let hidden = match self {
            Duckdb::Postgres(_) => "'information_schema', 'pg_catalog'",
            Duckdb::Mysql(_) => "'information_schema', 'mysql', 'sys', 'performance_schema'",
            Duckdb::Sqlite(_) | Duckdb::Files(_) => "''",
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT schema_name, table_name FROM duckdb_tables() \
             WHERE database_name = 'src' AND schema_name NOT IN ({hidden}) \
             ORDER BY 1, 2",
        ))?;
        let names = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        names.into_iter().map(|(schema, table)| {
            if schema.contains('.') || table.contains('.') {
                return Err(Error::InvalidArgument(format!(
                    "schema {schema:?}, table {table:?}: dots inside schema or table names are not supported"
                )));
            }
            Ok((format!("{schema}.{table}"), Some(table)))
        }).collect()
    }

    fn keys(&self, conn: &Connection) -> Result<HashMap<String, String>, Error> {
        let sql = match self {
            Duckdb::Files(_) => return Ok(HashMap::new()),
            Duckdb::Sqlite(_) => format!(
                "SELECT * FROM sqlite_query('src', '{}')",
                lit(r#"
                SELECT 'main.' || t.name AS object, c.name AS column_name, 0 AS priority
                FROM sqlite_master t JOIN pragma_table_info(t.name) c
                WHERE t.type = 'table' AND c.pk > 0
                  AND (SELECT count(*) FROM pragma_table_info(t.name) WHERE pk > 0) = 1
                  AND (c."notnull" = 1 OR (upper(c.type) = 'INTEGER'
                       AND NOT EXISTS (SELECT 1 FROM pragma_index_list(t.name) WHERE origin = 'pk')))
                UNION ALL
                SELECT 'main.' || t.name, min(c.name), 1
                FROM sqlite_master t JOIN pragma_index_list(t.name) i
                JOIN pragma_index_info(i.name) k JOIN pragma_table_info(t.name) c ON c.cid = k.cid
                WHERE t.type = 'table' AND i."unique" = 1 AND i.partial = 0
                GROUP BY t.name, i.name
                HAVING count(*) = 1 AND min(c."notnull") = 1
                   AND (SELECT count(*) FROM pragma_index_info(i.name)) = 1
                ORDER BY priority, column_name
            "#)
            ),
            Duckdb::Postgres(_) => format!(
                "SELECT * FROM postgres_query('src', '{}')",
                lit("SELECT n.nspname || '.' || t.relname, a.attname \
                 FROM pg_catalog.pg_index i \
                 JOIN pg_catalog.pg_class t ON t.oid = i.indrelid \
                 JOIN pg_catalog.pg_namespace n ON n.oid = t.relnamespace \
                 JOIN pg_catalog.pg_attribute a ON a.attrelid = t.oid AND a.attnum = i.indkey[0] \
                 WHERE i.indisunique AND i.indisvalid AND i.indnkeyatts = 1 \
                   AND i.indpred IS NULL AND i.indexprs IS NULL AND a.attnotnull \
                 ORDER BY i.indisprimary DESC, a.attname")
            ),
            Duckdb::Mysql(_) => String::from(
                "SELECT s.table_schema || '.' || s.table_name, min(s.column_name) AS column_name \
                 FROM src.information_schema.statistics s \
                 LEFT JOIN src.information_schema.columns c \
                   ON c.table_schema = s.table_schema AND c.table_name = s.table_name \
                  AND c.column_name = s.column_name \
                 WHERE s.non_unique = 0 \
                 GROUP BY s.table_schema, s.table_name, s.index_name \
                 HAVING count(*) = 1 AND min(c.is_nullable) = 'NO' \
                 ORDER BY CASE WHEN s.index_name = 'PRIMARY' THEN 0 ELSE 1 END, column_name",
            ),
        };
        let transaction = conn.unchecked_transaction()?;
        let mut stmt = transaction.prepare(&sql)?;
        let mut keys = HashMap::new();
        for key in stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (table, column) = key?;
            keys.entry(table).or_insert(column);
        }
        Ok(keys)
    }

    pub async fn catalog(&self) -> Result<super::Catalog, Error> {
        let source = self.clone();
        spawn_blocking(move || -> Result<super::Catalog, Error> {
            let file = match &source {
                Duckdb::Files(file) => file.clone(),
                _ => None,
            };
            if let Some(file) = &file {
                file.validate()?;
            }
            let conn = source.connect(file.as_ref())?;
            // (object, collection hint, what to SELECT from)
            let objects: Vec<(String, Option<String>, Select)> = match &file {
                // A file source's one object is the uri it was given.
                Some(file) => vec![(file.path.clone(), stem(&file.path), Select::file(file))],
                None => source
                    .list(&conn)?
                    .into_iter()
                    .map(|(from, hint)| {
                        let select = Select::table(&from);
                        (from, hint, select)
                    })
                    .collect(),
            };
            let mut keys = source.keys(&conn)?;
            let mut tables = Vec::with_capacity(objects.len());
            for (from, collection_hint, select) in objects {
                let mut stmt = conn
                    .prepare(&select.limit(Some(0)).into_sql())
                    .map_err(|e| read_error(&from, None, e))?;
                let schema = stmt
                    .query_arrow([])
                    .map_err(|e| read_error(&from, None, e))?
                    .get_schema();
                let columns: Vec<_> = schema
                    .fields()
                    .iter()
                    .map(|column| {
                        let field = arrow::ty(column.data_type());
                        if field.is_none() {
                            tracing::warn!(
                                "{from}: skipping column {:?}: {} has no equivalent in TopK",
                                column.name(),
                                column.data_type()
                            );
                        }
                        (column.name().clone(), field)
                    })
                    .collect();

                if columns.is_empty() {
                    return Err(Error::InvalidArgument(format!(
                        "no columns discovered for {from}"
                    )));
                }
                let key = keys.remove(&from);
                tables.push(Table {
                    from,
                    collection_hint,
                    columns,
                    key,
                });
            }
            Ok(super::Catalog { tables })
        })
        .await?
    }

    pub fn table(
        &self,
        target: Target,
        key: String,
        after: Option<Cursor>,
    ) -> Result<ChunkStream, Error> {
        let after = match after {
            None => None,
            Some(Cursor::Sql { column, value }) if column == key => Some(value),
            Some(Cursor::Key(value)) if target.id == key => Some(value),
            Some(cursor) => {
                return Err(Error::InvalidArgument(format!(
                "resume cursor {cursor:?} does not match source key {key:?}; start a new import"
            )))
            }
        };
        Ok(self.stream(move |source, tx| {
            let conn = source.connect(None)?;
            Plan::table(source, &target, &key, after.as_deref()).read(&conn, tx)
        }))
    }

    pub fn files(&self, target: Target, after: Option<Cursor>) -> Result<ChunkStream, Error> {
        let resume = match after {
            None => None,
            Some(Cursor::Offset { part, rows }) => Some((part, rows)),
            Some(cursor) => {
                return Err(Error::InvalidArgument(format!(
                    "resume cursor {cursor:?} does not fit {}",
                    target.from
                )))
            }
        };
        let file: File = target.from.parse()?;
        Ok(self.stream(move |source, tx| read::files(source, &file, &target, resume, tx)))
    }

    fn stream<T: Send + 'static>(
        &self,
        produce: impl FnOnce(&Duckdb, &Sender) -> Result<T, Error> + Send + 'static,
    ) -> ChunkStream {
        let source = self.clone();
        // One message per arrow batch; per-row sends make the channel the bottleneck.
        let (tx, mut rx) = mpsc::channel(2);
        Box::pin(async_stream::try_stream! {
            let reader = spawn_blocking(move || produce(&source, &tx));
            while let Some(chunk) = rx.recv().await {
                yield chunk;
            }
            reader.await??;
        })
    }
}

pub(super) fn extension_error(ext: &str, e: duckdb::Error) -> Error {
    Error::InvalidArgument(format!(
        "cannot load the duckdb {ext} extension (downloaded on first use — are you offline?): {e}"
    ))
}

pub(super) fn read_error(from: &str, filter: Option<&str>, e: duckdb::Error) -> Error {
    Error::InvalidArgument(match filter {
        Some(filter) => format!("reading {from}: {e} (filter: {filter:?})"),
        None => format!("reading {from}: {e}"),
    })
}
