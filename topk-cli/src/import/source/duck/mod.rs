mod arrow;
mod creds;
mod read;
mod select;

use std::ops::ControlFlow;
use std::path::Path;

use duckdb::Connection;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use topk_rs::proto::v1::control::FieldSpec;
use tracing::debug;

use crate::import::error::Error;
use crate::import::plan::Target;

use super::uri::SourceUrl;
use super::{ChunkStream, Cursor, Readers, Resume, Table};

use super::file::local_path;
use super::{File, Format, SourceLocation};
pub use creds::aws_process_profile;
use creds::secret;
use read::{Plan, Sender};
use select::Select;

#[derive(Clone, Debug)]
pub enum Duckdb {
    Postgres { url: SourceUrl, object: String },
    Mysql { url: SourceUrl, object: String },
    Sqlite { path: String, object: String },
    Files(File),
}

/// A single-quoted SQL literal's body.
pub(super) fn lit(value: &str) -> String {
    value.replace('\'', "''")
}

/// Per-reader duckdb budget. duckdb's default is 80% of RAM *per connection*,
/// which concurrent readers multiply into an OOM-kill; a 1.3 GiB single-row-group
/// parquet needs the whole column chunk resident.
const READER_MEMORY: u64 = 4 << 30;

impl Duckdb {
    /// A connection able to read the files, or attached to the database.
    fn connect(&self) -> Result<Connection, Error> {
        let conn = Connection::open_in_memory()?;
        // One thread, in order: rows arrive as stored, which makes a row offset
        // a resume point; more threads measured the same. A file is read once
        // per connection, so the remote file cache only keeps a copy of it.
        conn.execute_batch(&format!(
            "SET preserve_insertion_order = true; SET memory_limit = '{}MiB'; \
             SET threads = 1; SET enable_external_file_cache = false; SET temp_directory = '{}';",
            READER_MEMORY >> 20,
            lit(&std::env::temp_dir()
                .join("topk-import")
                .display()
                .to_string())
        ))?;
        let install = |ext: &str| {
            debug!(extension = ext, "loading DuckDB extension");
            conn.execute_batch(&format!("INSTALL {ext}; LOAD {ext};"))
                .map_err(|e| extension_error(ext, e))
        };
        // Community extensions live in a separate repo; a duckdb bump can 404 one.
        let install_community = |ext: &str| {
            debug!(extension = ext, "loading DuckDB community extension");
            conn.execute_batch(&format!("INSTALL {ext} FROM community; LOAD {ext};"))
                .map_err(|e| extension_error(ext, e))
        };

        let (ext, dsn) = match self {
            Duckdb::Postgres { url, .. } => ("postgres", url.0.as_str()),
            Duckdb::Mysql { url, .. } => ("mysql", url.0.as_str()),
            Duckdb::Sqlite { path, .. } => ("sqlite", path.as_str()),
            Duckdb::Files(file) => {
                match file.format {
                    Format::Avro => install("avro")?,
                    Format::Arrow => install_community("arrow")?,
                    Format::Json | Format::Parquet => {}
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
                Duckdb::Postgres { url, .. } | Duckdb::Mysql { url, .. } => url.to_string(),
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
            Duckdb::Postgres { .. } => Select::table(from).pushdown("src"),
            _ => Select::table(from),
        }
    }

    fn key(&self, conn: &Connection, schema: &str, table: &str) -> Result<Option<String>, Error> {
        let sql = match self {
            Duckdb::Files(_) => return Ok(None),
            Duckdb::Sqlite { .. } => format!(
                "SELECT * FROM sqlite_query('src', '{}')",
                lit(&format!(r#"
                SELECT c.name AS column_name, 0 AS priority
                FROM sqlite_master t JOIN pragma_table_info(t.name) c
                WHERE t.type = 'table' AND t.name = '{}' AND c.pk > 0
                  AND (SELECT count(*) FROM pragma_table_info(t.name) WHERE pk > 0) = 1
                  AND (c."notnull" = 1 OR (upper(c.type) = 'INTEGER'
                       AND NOT EXISTS (SELECT 1 FROM pragma_index_list(t.name) WHERE origin = 'pk')))
                UNION ALL
                SELECT min(c.name), 1
                FROM sqlite_master t JOIN pragma_index_list(t.name) i
                JOIN pragma_index_info(i.name) k JOIN pragma_table_info(t.name) c ON c.cid = k.cid
                WHERE t.type = 'table' AND t.name = '{}' AND i."unique" = 1 AND i.partial = 0
                GROUP BY t.name, i.name
                HAVING count(*) = 1 AND min(c."notnull") = 1
                   AND (SELECT count(*) FROM pragma_index_info(i.name)) = 1
                ORDER BY priority, column_name
            "#, lit(table), lit(table)))
            ),
            Duckdb::Postgres { .. } => format!(
                "SELECT * FROM postgres_query('src', '{}')",
                lit(&format!("SELECT a.attname \
                 FROM pg_catalog.pg_index i \
                 JOIN pg_catalog.pg_class t ON t.oid = i.indrelid \
                 JOIN pg_catalog.pg_namespace n ON n.oid = t.relnamespace \
                 JOIN pg_catalog.pg_attribute a ON a.attrelid = t.oid AND a.attnum = i.indkey[0] \
                 WHERE n.nspname = '{}' AND t.relname = '{}' AND i.indisunique AND i.indisvalid AND i.indnkeyatts = 1 \
                   AND i.indpred IS NULL AND i.indexprs IS NULL AND a.attnotnull \
                 ORDER BY i.indisprimary DESC, a.attname", lit(schema), lit(table)))
            ),
            Duckdb::Mysql { .. } => format!(
                "SELECT min(s.column_name) AS column_name \
                 FROM src.information_schema.statistics s \
                 LEFT JOIN src.information_schema.columns c \
                   ON c.table_schema = s.table_schema AND c.table_name = s.table_name \
                  AND c.column_name = s.column_name \
                 WHERE s.table_schema = '{}' AND s.table_name = '{}' AND s.non_unique = 0 \
                 GROUP BY s.table_schema, s.table_name, s.index_name \
                 HAVING count(*) = 1 AND min(c.is_nullable) = 'NO' \
                 ORDER BY CASE WHEN s.index_name = 'PRIMARY' THEN 0 ELSE 1 END, column_name", lit(schema), lit(table),
            ),
        };
        let transaction = conn.unchecked_transaction()?;
        let mut stmt = transaction.prepare(&sql)?;
        Ok(stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .next()
            .transpose()?)
    }

    /// The files a glob matches, in byte order; a plain path is its own list.
    fn list(&self, file: &File) -> Result<Vec<String>, Error> {
        let path = file.glob();
        if local_path(&path).is_some() {
            return Ok(vec![path]);
        }
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT file FROM glob('{}') ORDER BY 1",
                lit(&path)
            ))
            .map_err(|e| read_error(&SourceLocation::from(file), None, e))?;
        let files = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()
            .map_err(|e| read_error(&SourceLocation::from(file), None, e))?;
        Ok(files)
    }

    /// A glob over more than `sample` json or parquet files is described by `sample`
    /// of them, evenly spread from the first to the last.
    pub async fn discover(&self, sample: usize) -> Result<Table, Error> {
        let source = self.clone();
        spawn_blocking(move || -> Result<Table, Error> {
            let file = match &source {
                Duckdb::Files(file) => Some(file),
                _ => None,
            };
            if let Some(file) = file { file.validate()?; }
            let conn = source.connect()?;
            // A glob's schema is the union of its files' schemas, read one file per thread; order does not matter here.
            if file.is_some() { conn.execute_batch("SET threads = 16;")?; }
            let mut complete = true;
            let (from, select, key) = match &source {
                Duckdb::Files(file) => {
                    let mut files = vec![file.glob()];
                    if matches!(file.format, Format::Json | Format::Parquet) {
                        let (all, sample) = (source.list(file)?, sample.max(2));
                        if all.len() > sample {
                            complete = false;
                            files = (0..sample).map(|i| all[i * (all.len() - 1) / (sample - 1)].clone()).collect();
                        }
                    }
                    (SourceLocation::from(file), Select::file(file.format, &files), None)
                }
                Duckdb::Postgres { object, .. } | Duckdb::Mysql { object, .. } | Duckdb::Sqlite { object, .. } => {
                    let hidden = match &source {
                        Duckdb::Postgres { .. } => "'information_schema', 'pg_catalog'",
                        Duckdb::Mysql { .. } => "'information_schema', 'mysql', 'sys', 'performance_schema'",
                        _ => "''",
                    };
                    let mut stmt = conn.prepare(&format!(
                        "SELECT schema_name, table_name FROM duckdb_tables() \
                         WHERE database_name = 'src' AND schema_name NOT IN ({hidden}) \
                         AND (schema_name || '.' || table_name = ? OR table_name = ?) ORDER BY 1, 2"
                    ))?;
                    let names = stmt.query_map([object, object], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
                        .collect::<Result<Vec<_>, _>>()?;
                    let (schema, table) = match names.as_slice() {
                        [(schema, table)] => (schema, table),
                        [] => {
                            let mut stmt = conn.prepare(&format!(
                                "SELECT schema_name || '.' || table_name FROM duckdb_tables() \
                                 WHERE database_name = 'src' AND schema_name NOT IN ({hidden}) ORDER BY 1 LIMIT 21"
                            ))?;
                            let mut tables = stmt.query_map([], |row| row.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?;
                            if tables.len() > 20 { tables[20] = "…".into(); }
                            return Err(Error::InvalidArgument(match tables.is_empty() {
                                true => format!("source object {object:?} was not found: the database has no tables"),
                                false => format!("source object {object:?} was not found — available: {}", tables.join(", ")),
                            }));
                        }
                        _ => return Err(Error::InvalidArgument(format!("source object {object:?} is ambiguous; qualify it with its schema"))),
                    };
                    if schema.contains('.') || table.contains('.') {
                        return Err(Error::InvalidArgument(format!("schema {schema:?}, table {table:?}: dots inside schema or table names are not supported")));
                    }
                    let from = format!("{schema}.{table}");
                    let select = Select::table(&from);
                    (SourceLocation::Object(from), select, source.key(&conn, schema, table)?)
                }
            };
            let mut stmt = conn.prepare(&select.limit(Some(0)).into_sql())
                .map_err(|e| read_error(&from, None, e))?;
            let schema = stmt.query_arrow([]).map_err(|e| read_error(&from, None, e))?.get_schema();
            let mut columns: Vec<_> = schema.fields().iter()
                .map(|column| (column.name().clone(), arrow::ty(column.data_type()).unwrap_or_default())).collect();
            if columns.is_empty() {
                return Err(Error::InvalidArgument(format!("no columns discovered for {from}")));
            }
            for (name, _) in file.into_iter().flat_map(|file| &file.path_columns) {
                if let Some((taken, _)) = columns.iter().find(|(column, _)| column.eq_ignore_ascii_case(name)) {
                    return Err(Error::InvalidArgument(format!(
                        "{{{name}}} in {from}: the files already have a {taken:?} column; give the path column another name"
                    )));
                }
                columns.push((name.clone(), FieldSpec::text(false)));
            }
            Ok(Table { from, columns, key, complete })
        }).await?
    }

    pub fn table(
        &self,
        target: Target,
        key: String,
        after: Option<Cursor>,
        readers: Readers,
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
            let conn = source.connect()?;
            Plan::table(source, &target, &key, after.as_deref()).read(
                &conn,
                tx,
                readers.chunk_bytes,
            )
        }))
    }

    pub fn files(
        &self,
        file: File,
        target: Target,
        after: Option<Resume>,
        readers: Readers,
    ) -> Result<ChunkStream, Error> {
        let resume = after
            .map(|resume| {
                let open = resume
                    .open
                    .into_iter()
                    .map(|cursor| match cursor {
                        Cursor::Offset { part, rows } => Ok((part, rows)),
                        cursor => Err(Error::InvalidArgument(format!(
                            "resume cursor {cursor:?} does not fit {}",
                            target.from
                        ))),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok::<_, Error>((open, resume.done))
            })
            .transpose()?;
        Ok(self.stream(move |source, tx| read::files(source, &file, &target, resume, readers, tx)))
    }

    fn stream<T: Send + 'static>(
        &self,
        produce: impl FnOnce(&Duckdb, &Sender) -> Result<T, Error> + Send + 'static,
    ) -> ChunkStream {
        let source = self.clone();
        // One message per arrow batch; per-row sends make the channel the bottleneck.
        let (tx, mut rx) = mpsc::channel(2);
        Box::pin(async_stream::try_stream! {
            let mut reader = spawn_blocking(move || produce(&source, &tx));
            loop {
                let next = tokio::select! {
                    biased;
                    result = &mut reader => ControlFlow::Break(result),
                    chunk = rx.recv() => ControlFlow::Continue(chunk),
                };
                match next {
                    ControlFlow::Break(result) => {
                        result??;
                        while let Some(chunk) = rx.recv().await { yield chunk; }
                        break;
                    }
                    ControlFlow::Continue(Some(chunk)) => yield chunk,
                    ControlFlow::Continue(None) => {
                        reader.await??;
                        break;
                    }
                }
            }
        })
    }
}

pub(super) fn extension_error(ext: &str, e: duckdb::Error) -> Error {
    Error::InvalidArgument(format!(
        "cannot load the duckdb {ext} extension (downloaded on first use — are you offline?): {e}"
    ))
}

/// duckdb's message without the query it failed in, naming the source once.
pub(super) fn read_error(from: &SourceLocation, filter: Option<&str>, e: duckdb::Error) -> Error {
    let e = e.to_string();
    let mut message = e.split("\n\nLINE ").next().unwrap_or(&e).trim().to_string();
    if !message.contains(&*from.value()) {
        message = format!("reading {from}: {message}");
    }
    if let Some(filter) = filter {
        message.push_str(&format!(" (filter: {filter:?})"));
    }
    // A private or gated dataset answers 401 or 404 without a token.
    if from.value().starts_with("hf://")
        && std::env::var_os("HF_TOKEN").is_none()
        && !Path::new(&*shellexpand::tilde("~/.cache/huggingface/token")).exists()
    {
        message.push_str("; a private or gated dataset needs HF_TOKEN or `hf auth login`");
    }
    Error::InvalidArgument(message)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::TryStreamExt;
    use rstest::rstest;
    use tokio::sync::oneshot;

    use super::*;

    #[rstest]
    #[case::source_error(false)]
    #[case::reader_panic(true)]
    #[tokio::test]
    async fn reader_failure_releases_stream(#[case] panic: bool) {
        let (release, blocked) = oneshot::channel::<()>();
        let (ready, started) = std::sync::mpsc::channel();
        let source = Duckdb::Sqlite {
            path: String::new(),
            object: String::new(),
        };
        let mut stream = source.stream(move |_, tx| {
            let tx = tx.clone();
            let mut readers = tokio::task::JoinSet::new();
            readers.spawn_blocking(move || {
                ready.send(()).unwrap();
                let _ = blocked.blocking_recv();
                drop(tx);
            });
            started.recv().unwrap();
            if panic {
                panic!("reader panicked");
            }
            Err::<(), _>(Error::InvalidArgument("reader failed".into()))
        });
        let result = tokio::time::timeout(Duration::from_secs(1), stream.try_next()).await;
        drop(release);
        drop(stream);
        match result.unwrap() {
            Err(Error::Join(error)) if panic => assert!(error.is_panic()),
            Err(Error::InvalidArgument(message)) if !panic => assert_eq!(message, "reader failed"),
            _ => panic!("reader failure was not propagated"),
        }
    }

    #[tokio::test]
    async fn successful_reader_drains_chunks() {
        let source = Duckdb::Sqlite {
            path: String::new(),
            object: String::new(),
        };
        let chunks: Vec<_> = source
            .stream(|_, tx| {
                for rows in 0..10 {
                    tx.blocking_send(super::super::Chunk {
                        rows: Vec::new(),
                        cursor: Some(Cursor::Offset {
                            part: "a".into(),
                            rows,
                        }),
                        last: rows == 9,
                    })
                    .unwrap();
                }
                Ok(())
            })
            .try_collect()
            .await
            .unwrap();
        assert_eq!(chunks.len(), 10);
        assert!(chunks.last().unwrap().last);
    }
}
