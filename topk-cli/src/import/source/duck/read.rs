use duckdb::Connection;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use crate::import::document::id_string;
use crate::import::error::Error;
use crate::import::plan::Target;
use crate::import::source::{Chunk, Cursor, File, Readers, Record, SourceLocation};

use super::arrow;
use super::select::Select;
use super::{read_error, Duckdb};

pub(super) type Sender = mpsc::Sender<Chunk>;

/// A rendered query and what the reader needs alongside it: the source to name
/// in an error, its filter for context, and how the cursor advances.
pub(super) struct Plan {
    sql: String,
    from: SourceLocation,
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
                Some(Cursor::Sql {
                    column: column.clone(),
                    value: id_string(column, value.clone()).ok()?,
                })
            }
            Position::Offset(start) => Some(Cursor::Offset {
                part: from.to_string(),
                rows: start + read,
            }),
        }
    }
}

/// Files in glob order, one query each, `settings.count` of them at once; their chunks are
/// forwarded as they come. A cursor is `<file>:<rows read>`. Resuming `OFFSET`s into the
/// open files, and skips the files before the first of them and the `done` ones.
pub(super) fn files(
    source: &Duckdb,
    file: &File,
    target: &Target,
    resume: Option<(Vec<(String, u64)>, Vec<String>)>,
    settings: Readers,
    tx: &Sender,
) -> Result<(), Error> {
    let mut files = source.list(file)?;
    if let Some(shard) = settings.shard {
        files.retain(|file| shard.takes(file));
    }
    // Globs list in byte order, which is how the cursor compares.
    let mut planned = files.into_iter().filter_map(|file| match &resume {
        None => Some((file, 0)),
        Some((open, done)) => match open.iter().find(|(part, _)| *part == file) {
            Some((_, rows)) => Some((file, *rows)),
            None if open.first().is_some_and(|(first, _)| file < *first) => None,
            None if done.contains(&file) => None,
            None => Some((file, 0)),
        },
    });
    // A limit is a running budget: the next query depends on the previous count.
    let count = match target.limit {
        Some(_) => 1,
        None => settings.count,
    };
    let mut remaining = target.limit;
    let mut readers = JoinSet::new();
    loop {
        while readers.len() < count && remaining != Some(0) {
            let (path, offset) = match planned.next() {
                Some(next) => next,
                None => break,
            };
            // The consumer learns of the file before a later file's rows reach it.
            let open = Chunk {
                rows: Vec::new(),
                cursor: Some(Cursor::Offset {
                    part: path.clone(),
                    rows: offset,
                }),
                last: false,
            };
            if tx.blocking_send(open).is_err() {
                return Ok(());
            }
            let plan = Plan::file(file, path.clone(), target, offset, remaining)?;
            let (source, tx) = (source.clone(), tx.clone());
            readers.spawn_blocking(move || {
                let read = plan.read(&source.connect()?, &tx, settings.chunk_bytes)?;
                Ok::<_, Error>((path, offset, read))
            });
        }
        let (part, offset, read) = match Handle::current().block_on(readers.join_next()) {
            Some(reader) => reader??,
            None => break,
        };
        // A file the limit cut short has rows left.
        if remaining.is_none_or(|n| read < n) {
            let end = Cursor::Offset {
                part,
                rows: offset + read,
            };
            let last = Chunk {
                rows: Vec::new(),
                cursor: Some(end),
                last: true,
            };
            if tx.blocking_send(last).is_err() {
                return Ok(());
            }
        }
        if let Some(n) = remaining.as_mut() {
            *n = n.saturating_sub(read);
        }
    }
    Ok(())
}

impl Plan {
    /// An attached table ordered by the id column, so a batch's last id is a
    /// resume point.
    pub(super) fn table(source: &Duckdb, target: &Target, key: &str, after: Option<&str>) -> Plan {
        let mut columns = target.source_columns();
        columns.insert(key);
        // The resume key is fetched alongside the mapped source columns.
        // A table has one validated schema, and for postgres the list has to be
        // plain anyway: the query runs on the server.
        let sql = source
            .select(&target.from.value())
            .columns(columns)
            .filter(target.filter.as_deref())
            .after(key, after)
            .order_by(key)
            .limit(target.limit)
            .into_sql();
        Plan {
            sql,
            from: target.from.clone(),
            filter: target.filter.clone(),
            position: Position::Key(key.to_string()),
        }
    }

    /// One file of a glob, resumed at `offset`. A column only some files carry
    /// must stay absent from the rest the way `union_by_name` leaves it, rather
    /// than raise a binder error: `COLUMNS` keeps the names that exist.
    fn file(
        file: &File,
        path: String,
        target: &Target,
        offset: u64,
        limit: Option<u64>,
    ) -> Result<Plan, Error> {
        Ok(Plan {
            sql: Select::file(file.format, std::slice::from_ref(&path))
                .constants(file.path_values(&path)?)
                .except(target.exclude.iter().map(String::as_str))
                .filter(target.filter.as_deref())
                .limit(limit)
                .offset(offset)
                .into_sql(),
            from: match file.store {
                None => SourceLocation::Local(path.into()),
                Some(_) => SourceLocation::Remote(path),
            },
            filter: target.filter.clone(),
            position: Position::Offset(offset),
        })
    }

    pub(super) fn read(
        &self,
        conn: &Connection,
        tx: &Sender,
        chunk_bytes: u64,
    ) -> Result<u64, Error> {
        self.execute(conn, tx, chunk_bytes)
            .map_err(|e| read_error(&self.from, self.filter.as_deref(), e))
    }

    /// Returns the rows read; stops without error if the receiver goes away. A batch is cut
    /// into chunks of about `chunk_bytes`, its rows taken to be equally large.
    fn execute(
        &self,
        conn: &Connection,
        tx: &Sender,
        chunk_bytes: u64,
    ) -> Result<u64, duckdb::Error> {
        let mut stmt = conn.prepare(&self.sql)?;
        // `query_arrow` materializes the whole result; `stream_arrow` streams.
        // Stepped by hand: the iterator panics on a mid-stream failure, `step`
        // returns it.
        let _ = stmt.stream_arrow([])?;
        let mut read = 0;
        while let Some(array) = stmt.step()? {
            let batch = duckdb::arrow::record_batch::RecordBatch::from(&array);
            let schema = batch.schema();
            let total = batch.num_rows();
            let step = (chunk_bytes.saturating_mul(total as u64)
                / (batch.get_array_memory_size() as u64).max(1))
            .clamp(1, total.max(1) as u64) as usize;
            for start in (0..total).step_by(step) {
                let rows: Vec<Result<Record, Error>> = (start..total.min(start + step))
                    .map(|row| {
                        schema
                            .fields()
                            .iter()
                            .zip(batch.columns())
                            .map(|(field, array)| {
                                Ok((field.name().clone(), arrow::value(array, row)?))
                            })
                            .collect::<Result<Record, Error>>()
                    })
                    .collect();
                read += rows.len() as u64;
                let mark = self.position.advance(&self.from.value(), read, &rows);
                let chunk = Chunk {
                    rows,
                    cursor: mark,
                    last: false,
                };
                if tx.blocking_send(chunk).is_err() {
                    return Ok(read);
                }
            }
        }
        Ok(read)
    }
}
