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
use super::select::{Plan, Position, Select};
use super::{lit, quoted, read_error, Duckdb};

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
    let id = target.id.as_deref().unwrap_or(ID);
    let relation = match source {
        Duckdb::Postgres(_) => quoted(&target.from, '"'),
        _ => format!("src.{}", quoted(&target.from, '"')),
    };
    // The spec is a whitelist, so a column no field reads is never fetched.
    // A table has one validated schema, and for postgres the list has to be
    // plain anyway: the query runs on the server.
    let select = match source {
        Duckdb::Postgres(_) => Select::postgres(relation),
        _ => Select::from(relation),
    }
    .reading(target.from.clone())
    .columns(target.columns())
    .filter(target.filter.clone())
    .limit(target.limit);
    let plan = select.by_id(id, after);
    read(conn, plan, tx).map(|_| ())
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
pub(super) const READ_AHEAD: usize = 1;

/// Files in glob order, one query each, read `READ_AHEAD` deep and forwarded in
/// order. The cursor is `<file>:<rows read>`: resuming skips files before it and
/// `OFFSET`s into the one it names.
pub(super) fn files(
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
    // A glob is read one file at a time, so a column only some files carry
    // must stay absent from the rest the way `union_by_name` leaves it,
    // rather than raise a binder error: `COLUMNS` keeps the names that exist.
    let plan = Select::from(reader(file))
        .reading(file.path.clone())
        .existing_columns(target.columns())
        .filter(target.filter.clone())
        .limit(limit)
        .by_offset(offset);
    read(&conn, plan, tx)
}

/// Executes one planned query. Returns the rows read and stops without error
/// if the receiver goes away.
fn read(conn: &Connection, mut plan: Plan, tx: &Sender) -> Result<u64, Error> {
    let sql = plan.to_string();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| read_error(&plan.from, plan.filter.as_deref(), e))?;
    // `query_arrow` materializes the whole result; `stream_arrow` streams.
    // Stepped by hand: the iterator panics on a mid-stream failure, `step`
    // returns it.
    let _ = stmt
        .stream_arrow([])
        .map_err(|e| read_error(&plan.from, plan.filter.as_deref(), e))?;
    let mut read = 0;
    while let Some(array) = stmt
        .step()
        .map_err(|e| read_error(&plan.from, plan.filter.as_deref(), e))?
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
        let mark = plan.position.advance(&plan.from, &rows);
        if tx.blocking_send(Ok(Chunk { rows, cursor: mark })).is_err() {
            return Ok(read);
        }
    }
    Ok(read)
}
