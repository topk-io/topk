use std::path::Path;

use duckdb::arrow::array::ArrayRef;
use duckdb::Connection;
use futures::StreamExt;
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::ReceiverStream;

use crate::import::error::Error;
use crate::import::source::codec::arrow;
use crate::import::source::uri::{Format, ObjectStore, Uri};
use crate::import::spec::{Field, Target};

use super::{Record, Records, Table};

#[derive(Clone)]
pub enum Duckdb {
    Postgres(String),
    Mysql(String),
    Sqlite(String),
    /// Path-free: each `target.from` is its own locator, resolved at read time,
    /// so one source serves a whole multi-file spec.
    Files,
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
    path.file_stem().and_then(|s| s.to_str()).map(str::to_string)
}

impl Duckdb {
    /// `from` parsed as a file locator — the only `from` a file source can read.
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

    /// A connection able to read `from` — only Files derives anything from it;
    /// attached sources carry their DSN.
    fn connect(&self, from: &str) -> Result<Connection, Error> {
        let conn = Connection::open_in_memory()?;
        let install = |ext: &str| {
            conn.execute_batch(&format!("INSTALL {ext}; LOAD {ext};"))
                .map_err(|e| extension_error(ext, e))
        };

        let (ext, dsn) = match self {
            Duckdb::Postgres(url) => ("postgres", url),
            Duckdb::Mysql(url) => ("mysql", url),
            Duckdb::Sqlite(path) => ("sqlite", path),
            Duckdb::Files => {
                let (_, store, format) = Self::file(from)?;
                match format {
                    Format::Avro => install("avro")?,
                    Format::Xlsx => install("excel")?,
                    Format::Csv | Format::Json | Format::Parquet => {}
                }
                if let Some(store) = store {
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
                let reader = match format {
                    Format::Csv => "read_csv_auto",
                    Format::Json => "read_json_auto",
                    Format::Parquet => "read_parquet",
                    Format::Avro => "read_avro",
                    Format::Xlsx => "read_xlsx",
                };
                format!("{reader}('{}')", path.replace('\'', "''"))
            }
            _ => {
                let path: Vec<String> = from
                    .split('.')
                    .map(|part| format!("\"{}\"", part.replace('"', "\"\"")))
                    .collect();
                format!("src.{}", path.join("."))
            }
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
            .query_map([], |r| Ok((r.get::<_, String>(0)?, Some(r.get::<_, String>(1)?))))?
            .collect::<Result<Vec<(String, Option<String>)>, _>>()?;
        Ok(names)
    }

    /// The single-column primary key of `from`; None for composite keys.
    fn primary_key(&self, conn: &Connection, from: &str) -> Result<Option<String>, Error> {
        // duckdb_constraints() materializes constraints for every attached table
        // (pg_catalog included) before filtering — seconds per call on a real
        // database, and the mysql scanner exposes no PK rows through it at all.
        // Ask the source's own information_schema instead; sqlite has none, but
        // its catalogs are small enough for duckdb_constraints().
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
            // A file source's locator is the path — the same `from` that
            // stream() will read; attached sources carry their DSN and ignore it.
            let conn = source.connect(match &uri {
                Uri::File { path, .. } => path,
                _ => "",
            })?;
            // An empty file still reads as a single synthetic `column0`, which
            // becomes a confusing "no id column" error downstream.
            if let Uri::File { path, store: None, .. } = &uri {
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

    pub async fn stream(&self, target: &Target) -> Result<Records, Error> {
        let source = self.clone();
        let mut sql = format!("SELECT * FROM {}", source.table_ref(&target.from)?);
        if let Some(filter) = target.filter.as_deref() {
            sql.push_str(&format!(" WHERE ({filter})"));
        }
        if let Some(n) = target.limit {
            sql.push_str(&format!(" LIMIT {n}"));
        }
        // One message per arrow batch (~2k rows), not per row: the sink batches
        // by bytes anyway, and per-row sends make the channel the bottleneck.
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<Result<Record, Error>>>(2);
        let (from, filter) = (target.from.clone(), target.filter.clone());
        spawn_blocking(move || {
            let produce = || -> Result<(), Error> {
                let conn = source.connect(&from)?;
                let mut stmt = conn
                    .prepare(&sql)
                    .map_err(|e| read_error(&from, filter.as_deref(), e))?;
                for batch in stmt
                    .query_arrow([])
                    .map_err(|e| read_error(&from, filter.as_deref(), e))?
                {
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
                                .map(|(name, array)| {
                                    Ok((name.to_string(), arrow::value(array, row)?))
                                })
                                .collect::<Result<Record, Error>>()
                                .map_err(|e| Error::Row(Box::new(e)))
                        })
                        .collect();
                    if tx.blocking_send(rows).is_err() {
                        return Ok(());
                    }
                }
                Ok(())
            };
            if let Err(e) = produce() {
                let _ = tx.blocking_send(vec![Err(e)]);
            }
        });
        Ok(Box::pin(
            ReceiverStream::new(rx).flat_map(futures::stream::iter),
        ))
    }
}

/// Duckdb errors embed the generated SQL as a `LINE n: …` block; ours, not the
/// user's, so drop it.
fn strip_sql(e: &duckdb::Error) -> String {
    let msg = e.to_string();
    msg.split("\nLINE ").next().unwrap_or(&msg).trim().to_string()
}

fn extension_error(ext: &str, e: duckdb::Error) -> Error {
    Error::InvalidArgument(format!(
        "cannot load the duckdb {ext} extension (downloaded on first use — are you offline?): {}",
        strip_sql(&e)
    ))
}

fn secret(conn: &Connection, store: &ObjectStore) -> Result<(), Error> {
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
                     CHAIN 'env', REGION '{region}', ENDPOINT '{host}', URL_STYLE 'path', \
                     USE_SSL {use_ssl});"
                ))?;
            }
            Err(_) => conn.execute_batch(
                "CREATE OR REPLACE SECRET s3 (TYPE s3, PROVIDER credential_chain);",
            )?,
        },
        ObjectStore::Gcs => {
            conn.execute_batch(
                "CREATE OR REPLACE SECRET gcs (TYPE gcs, PROVIDER credential_chain);",
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
