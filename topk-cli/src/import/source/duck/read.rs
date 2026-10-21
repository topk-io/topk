use std::collections::VecDeque;
use std::thread::{self, JoinHandle};

use duckdb::arrow::array::ArrayRef;
use duckdb::Connection;
use tokio::sync::mpsc;

use crate::import::decode::id_string;
use crate::import::error::Error;
use crate::import::source::codec::arrow;
use crate::import::source::{Chunk, Cursor, Record};
use crate::import::spec::Target;

use super::file::{local_path, reader, File};
use super::select::Select;
use super::{lit, read_error, Duckdb};

pub(super) type Sender = mpsc::Sender<Chunk>;
type Receiver = mpsc::Receiver<Chunk>;

/// A rendered query and what the reader needs alongside it: the source to name
/// in an error, its filter for context, and how the cursor advances.
pub(super) struct Plan {
    sql: String,
    from: String,
    filter: Option<String>,
    position: Position,
}

/// How this part's cursor moves: by the last value of a column, or by rows read
/// within the part.
enum Position {
    Key(String),
    Offset(u64),
}

impl Position {
    fn advance(&self, from: &str, read: u64, rows: &[Result<Record, Error>]) -> Option<Cursor> {
        match self {
            Position::Key(column) => {
                let record = rows.last()?.as_ref().ok()?;
                let (_, value) = record.iter().find(|(key, _)| key == column)?;
                Some(Cursor::Key(id_string(column, value.clone()).ok()?))
            }
            Position::Offset(start) => Some(Cursor::Offset {
                part: from.to_string(),
                rows: start + read,
            }),
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
    file: &File,
    target: &Target,
    filter: Option<&str>,
    resume: Option<(String, u64)>,
    tx: &Sender,
) -> Result<(), Error> {
    let path = &file.path;
    let files: Vec<String> = match local_path(path).is_some() {
        true => vec![path.clone()],
        false => {
            let conn = source.connect(Some(file))?;
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
    // Globs list in byte order, which is how the cursor compares.
    let planned: Vec<(String, u64)> = files
        .into_iter()
        .filter_map(|file| match &resume {
            Some((done, _)) if file < *done => None,
            Some((done, rows)) if file == *done => Some((file, *rows)),
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
            let file = File {
                path,
                ..file.clone()
            };
            let plan = Plan::file(&file, target, filter, offset, remaining);
            readers.push_back((
                file_rx,
                thread::spawn(move || {
                    let conn = source.connect(Some(&file))?;
                    plan.read(&conn, &file_tx)
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

impl Plan {
    /// An attached table ordered by the id column, so a batch's last id is a
    /// resume point.
    pub(super) fn table(
        source: &Duckdb,
        target: &Target,
        filter: Option<&str>,
        after: Option<&str>,
    ) -> Plan {
        let id = target.id_column();
        // The spec is a whitelist, so a column no field reads is never fetched.
        // A table has one validated schema, and for postgres the list has to be
        // plain anyway: the query runs on the server.
        let sql = source
            .select(&target.from)
            .columns(target.source_columns())
            .filter(filter)
            .after(id, after)
            .order_by(id)
            .limit(target.limit)
            .into_sql();
        Plan {
            sql,
            from: target.from.clone(),
            filter: filter.map(str::to_string),
            position: Position::Key(id.to_string()),
        }
    }

    /// One file of a glob, resumed at `offset`. A column only some files carry
    /// must stay absent from the rest the way `union_by_name` leaves it, rather
    /// than raise a binder error: `COLUMNS` keeps the names that exist.
    fn file(
        file: &File,
        target: &Target,
        filter: Option<&str>,
        offset: u64,
        limit: Option<u64>,
    ) -> Plan {
        Plan {
            sql: Select::new(reader(file))
                .existing_columns(target.source_columns())
                .filter(filter)
                .limit(limit)
                .offset(offset)
                .into_sql(),
            from: file.path.clone(),
            filter: filter.map(str::to_string),
            position: Position::Offset(offset),
        }
    }

    pub(super) fn read(&self, conn: &Connection, tx: &Sender) -> Result<u64, Error> {
        self.execute(conn, tx)
            .map_err(|e| read_error(&self.from, self.filter.as_deref(), e))
    }

    /// Returns the rows read; stops without error if the receiver goes away.
    fn execute(&self, conn: &Connection, tx: &Sender) -> Result<u64, duckdb::Error> {
        let mut stmt = conn.prepare(&self.sql)?;
        // `query_arrow` materializes the whole result; `stream_arrow` streams.
        // Stepped by hand: the iterator panics on a mid-stream failure, `step`
        // returns it.
        let _ = stmt.stream_arrow([])?;
        let mut read = 0;
        while let Some(array) = stmt.step()? {
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
            let mark = self.position.advance(&self.from, read, &rows);
            if tx.blocking_send(Chunk { rows, cursor: mark }).is_err() {
                return Ok(read);
            }
        }
        Ok(read)
    }
}
