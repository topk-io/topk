use std::collections::VecDeque;
use std::thread::{self, JoinHandle};

use duckdb::arrow::array::ArrayRef;
use duckdb::Connection;
use tokio::sync::mpsc;

use crate::import::decode::id_string;
use crate::import::error::Error;
use crate::import::source::codec::arrow;
use crate::import::source::{Chunk, Record};
use crate::import::spec::Target;
use crate::import::ID;

use super::file::{local_path, reader, File};
use super::{lit, quoted, read_error, strip_sql, Duckdb, READER_MEMORY};

pub(super) type Sender = mpsc::Sender<Result<Chunk, Error>>;
type Receiver = mpsc::Receiver<Result<Chunk, Error>>;

/// An attached table ordered by the id column, so a batch's last id is a
/// resume point. Postgres sorts on its own key via `postgres_query`;
/// `mysql_query` breaks under a prepared statement ("Lost connection to
/// server during query"), so mysql sorts in duckdb like sqlite.
pub(super) fn table(
    source: &Duckdb,
    conn: &Connection,
    target: &Target,
    after: Option<&str>,
    tx: &Sender,
) -> Result<(), Error> {
    read(conn, Select::table(source, target, after), tx).map(|_| ())
}

/// A planned DuckDB query. Construction owns SQL generation and resume
/// semantics; execution only needs a connection and somewhere to send chunks.
struct Select {
    sql: String,
    from: String,
    filter: Option<String>,
    position: Position,
    /// The columns asked of the relation, for the error that names the widest.
    columns: Vec<String>,
}

impl Select {
    fn table(source: &Duckdb, target: &Target, after: Option<&str>) -> Select {
        let id = target.id.as_deref().unwrap_or(ID);
        let order = quoted(id, '"');
        let relation = match source {
            Duckdb::Postgres(_) => quoted(&target.from, '"'),
            _ => format!("src.{}", quoted(&target.from, '"')),
        };
        let predicates: Vec<String> = [
            target.filter.as_ref().map(|filter| format!("({filter})")),
            after.map(|after| format!("{order} > '{}'", lit(after))),
        ]
        .into_iter()
        .flatten()
        .collect();
        // The spec is a whitelist, so a column no field reads is never fetched.
        // A table has one validated schema, and for postgres the list has to be
        // plain anyway: the query runs on the server.
        let projection: Vec<String> = target
            .columns()
            .into_iter()
            .map(|column| quoted(column, '"'))
            .collect();
        let mut sql = format!("SELECT {} FROM {relation}", projection.join(", "));
        if !predicates.is_empty() {
            sql.push_str(&format!(" WHERE {}", predicates.join(" AND ")));
        }
        sql.push_str(&format!(" ORDER BY {order}"));
        if let Some(limit) = target.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        let sql = match source {
            Duckdb::Postgres(_) => {
                format!("SELECT * FROM postgres_query('src', '{}')", lit(&sql))
            }
            _ => sql,
        };
        Select {
            sql,
            from: target.from.clone(),
            filter: target.filter.clone(),
            position: Position::Id(id.to_string()),
            columns: target.columns().into_iter().map(str::to_string).collect(),
        }
    }

    fn file(file: &File, target: &Target, offset: u64, limit: Option<u64>) -> Select {
        // A glob is read one file at a time, so a column only some files carry
        // must stay absent from the rest the way `union_by_name` leaves it,
        // rather than raise a binder error: `COLUMNS` keeps the names that exist.
        let names: Vec<String> = target
            .columns()
            .into_iter()
            .map(|column| format!("'{}'", lit(column)))
            .collect();
        let mut sql = format!(
            "SELECT COLUMNS(c -> c IN [{}]) FROM {}",
            names.join(", "),
            reader(file)
        );
        if let Some(filter) = &target.filter {
            sql.push_str(&format!(" WHERE ({filter})"));
        }
        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        if offset > 0 {
            sql.push_str(&format!(" OFFSET {offset}"));
        }
        Select {
            sql,
            from: file.path.clone(),
            filter: target.filter.clone(),
            position: Position::Offset(offset),
            columns: target.columns().into_iter().map(str::to_string).collect(),
        }
    }
}

impl Select {
    /// duckdb's OOM leaks the engine at a public edge, and its advice does not
    /// apply: a reader holds a whole column chunk, a single-row-group file has
    /// no smaller unit, so `LIMIT` does not shrink it and a bigger machine does
    /// not help — `READER_MEMORY` is a constant. Name the column instead, with a
    /// footer read that only runs on the way to failing.
    fn error(&self, conn: &Connection, e: duckdb::Error) -> Error {
        let msg = strip_sql(&e);
        if !msg.contains("Out of Memory") && !msg.contains("could not allocate") {
            return read_error(&self.from, self.filter.as_deref(), e);
        }
        let widest = match widest_column(conn, &self.from, &self.columns) {
            Some((column, bytes)) => {
                format!("{column:?} needs {} resident", bytesize::ByteSize(bytes))
            }
            None => "a column is too wide to hold".to_string(),
        };
        Error::InvalidArgument(format!(
            "reading {}: {widest} and one reader is budgeted {READER_MEMORY}. \
             Drop the field from the spec to skip the column.",
            self.from
        ))
    }
}

/// The heaviest of `columns` in a parquet file, by its footer. Nested columns
/// are leaves under a dotted path, so they sum back to the name a spec uses;
/// nothing else answers, which is the whole point of it being optional.
fn widest_column(conn: &Connection, path: &str, columns: &[String]) -> Option<(String, u64)> {
    let wanted: Vec<String> = columns.iter().map(|c| format!("'{}'", lit(c))).collect();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, bytes FROM ( \
               SELECT split_part(path_in_schema, ',', 1) AS name, \
                      sum(total_compressed_size)::UBIGINT AS bytes \
               FROM parquet_metadata('{}') GROUP BY 1 \
             ) WHERE name IN [{}] ORDER BY bytes DESC LIMIT 1",
            lit(path),
            wanted.join(", ")
        ))
        .ok()?;
    stmt.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
        .ok()
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
/// read whole before its first row, so each file boundary stalls the sink. One
/// ahead was enough on s3 (29 → 41 MiB/s, and two measured the same), but a
/// higher-latency store is bound by round trips, not bandwidth: over hf:// the
/// same import runs 1234 → 2067 rows/s going from one ahead to seven. They share
/// the scan's connection, so the depth costs round trips, not another buffer pool.
const READ_AHEAD: usize = 7;

/// Files in glob order, one query each, read `READ_AHEAD` deep and forwarded in
/// order. The cursor is `<file>:<rows read>`: resuming skips files before it and
/// `OFFSET`s into the one it names.
pub(super) fn files(
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
                    lit(path)
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
            // Another connection to the same database: extensions and secrets are
            // already installed on it, and its buffer pool is shared, so reading
            // deeper does not multiply memory.
            let reader = conn.try_clone()?;
            let target = target.clone();
            let file = File {
                path,
                ..file.clone()
            };
            let limit = remaining;
            readers.push_back((
                file_rx,
                thread::spawn(move || read_file(reader, &file, &target, offset, limit, &file_tx)),
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
    conn: Connection,
    file: &File,
    target: &Target,
    offset: u64,
    limit: Option<u64>,
    tx: &Sender,
) -> Result<u64, Error> {
    read(&conn, Select::file(file, target, offset, limit), tx)
}

/// Executes one planned query. Returns the rows read and stops without error
/// if the receiver goes away.
fn read(conn: &Connection, mut select: Select, tx: &Sender) -> Result<u64, Error> {
    let mut stmt = conn
        .prepare(&select.sql)
        .map_err(|e| select.error(conn, e))?;
    // `query_arrow` materializes the whole result; `stream_arrow` streams.
    // Stepped by hand: the iterator panics on a mid-stream failure, `step`
    // returns it.
    let _ = stmt.stream_arrow([]).map_err(|e| select.error(conn, e))?;
    let mut read = 0;
    while let Some(array) = stmt.step().map_err(|e| select.error(conn, e))? {
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
        if tx.blocking_send(Ok(Chunk { rows, cursor: mark })).is_err() {
            return Ok(read);
        }
    }
    Ok(read)
}
