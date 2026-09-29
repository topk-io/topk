use std::io::{BufWriter, Write};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use clap::ArgGroup;
use clap_stdin::{FileOrStdin, MaybeStdin};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{ContentArrangement, Table};
use futures::{StreamExt, TryStreamExt};
use serde_json::{Map, Value};
use sqlx::postgres::PgRow;
use sqlx::{Column, Either, Postgres, Row, Type, ValueRef};
use tokio::time::timeout;

use crate::client::SqlClient;
use crate::config::Config;
use crate::endpoint::DataEndpoint;
use crate::output::{json_line, Output};

pub mod meta;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Text output prints a table per this many rows instead of holding the whole result.
const TABLE_ROWS: usize = 50;
const MIN_WIDTH: u16 = 20;

#[derive(clap::Args)]
#[command(group(ArgGroup::new("input").required(true).args(["query", "file"])))]
pub struct SqlArgs {
    /// Statements to run, separated by `;`, or `-` to read them from stdin
    query: Option<MaybeStdin<String>>,

    /// Read the statements from a file, or `-` for stdin
    #[arg(short, long)]
    file: Option<FileOrStdin>,

    #[command(flatten)]
    data: DataEndpoint,
}

impl SqlArgs {
    fn input_sql(&self) -> Result<String> {
        let sql = match (&self.query, &self.file) {
            (Some(query), _) => query.to_string(),
            (None, Some(file)) => file.clone().contents()?,
            (None, None) => bail!("pass the SQL as an argument or with --file"),
        };
        ensure!(!sql.trim().is_empty(), "no SQL to run");
        Ok(sql)
    }
}

pub async fn run(config: Config, args: &SqlArgs, output: Output) -> Result<ExitCode> {
    let sql = args.input_sql()?;
    let (sql, describes) = match meta::expand(&sql)? {
        Some(meta) => (meta.sql, meta.describes),
        None => (sql, None),
    };
    let mut client = timeout(CONNECT_TIMEOUT, SqlClient::connect(&config, &args.data))
        .await
        .with_context(|| format!("connecting timed out after {CONNECT_TIMEOUT:?}"))??;
    // Like psql, look the table up first rather than describing nothing.
    if let Some(table) = describes {
        let found = client
            .execute(&meta::table(&table))
            .try_fold(
                false,
                |found, item| async move { Ok(found || item.is_right()) },
            )
            .await?;
        ensure!(found, "did not find a table named {table:?}");
    }
    let mut printer = Printer::new(BufWriter::new(std::io::stdout().lock()), output);
    let mut results = client.execute(&sql);
    // Each statement's rows print as they arrive; an error stops the rest.
    while let Some(item) = results.next().await {
        match item? {
            Either::Right(row) => printer.row(row)?,
            Either::Left(done) => printer.done(done.rows_affected())?,
        }
    }
    Ok(ExitCode::SUCCESS)
}

struct Printer<W: Write> {
    out: W,
    output: Output,
    /// Text rows waiting for their table.
    table: Vec<PgRow>,
    /// Rows the current statement returned so far.
    rows: u64,
}

impl<W: Write> Printer<W> {
    fn new(out: W, output: Output) -> Self {
        Self {
            out,
            output,
            table: Vec::new(),
            rows: 0,
        }
    }

    fn row(&mut self, row: PgRow) -> Result<()> {
        self.rows += 1;
        match self.output {
            Output::Json => {
                let object = (0..row.len())
                    .map(|i| Ok((row.column(i).name().to_owned(), value(&row, i)?)))
                    .collect::<Result<Map<_, _>>>()?;
                json_line(&mut self.out, &object)?;
            }
            Output::Text => {
                self.table.push(row);
                if self.table.len() == TABLE_ROWS {
                    self.print_table()?;
                }
            }
        }
        Ok(())
    }

    fn done(&mut self, affected: u64) -> Result<()> {
        // Without rows an empty SELECT and a CREATE look the same, so the wording fits both.
        match (self.output, std::mem::take(&mut self.rows)) {
            (Output::Json, _) => {}
            (Output::Text, 0) => match affected {
                0 => writeln!(self.out, "(0 rows)")?,
                1 => writeln!(self.out, "OK, 1 row affected")?,
                n => writeln!(self.out, "OK, {n} rows affected")?,
            },
            (Output::Text, rows) => {
                self.print_table()?;
                match rows {
                    1 => writeln!(self.out, "(1 row)")?,
                    n => writeln!(self.out, "({n} rows)")?,
                }
            }
        }
        Ok(self.out.flush()?)
    }

    fn print_table(&mut self) -> Result<()> {
        let Some(first) = self.table.first() else {
            return Ok(());
        };
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_header(first.columns().iter().map(|column| column.name()));
        // Wrap to the terminal, unless it reports no usable width.
        if table.width().is_some_and(|width| width >= MIN_WIDTH) {
            table.set_content_arrangement(ContentArrangement::Dynamic);
        }
        for row in self.table.drain(..) {
            table.add_row(
                (0..row.len())
                    // As the server printed it; NULL is empty, as in psql.
                    .map(|i| {
                        Ok(row
                            .try_get_unchecked::<Option<String>, _>(i)?
                            .unwrap_or_default())
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
        }
        writeln!(self.out, "{table}")?;
        Ok(self.out.flush()?)
    }
}

/// Values arrive as text; scalars and JSON keep their type, everything else stays text.
fn value(row: &PgRow, i: usize) -> Result<Value> {
    if row.try_get_raw(i)?.is_null() {
        return Ok(Value::Null);
    }
    Ok(match row.column(i).type_info() {
        ty if <bool as Type<Postgres>>::compatible(ty) => row.try_get::<bool, _>(i)?.into(),
        ty if <i16 as Type<Postgres>>::compatible(ty) => row.try_get::<i16, _>(i)?.into(),
        ty if <i32 as Type<Postgres>>::compatible(ty) => row.try_get::<i32, _>(i)?.into(),
        ty if <i64 as Type<Postgres>>::compatible(ty) => row.try_get::<i64, _>(i)?.into(),
        ty if <f32 as Type<Postgres>>::compatible(ty) => row.try_get::<f32, _>(i)?.into(),
        ty if <f64 as Type<Postgres>>::compatible(ty) => row.try_get::<f64, _>(i)?.into(),
        ty if <Value as Type<Postgres>>::compatible(ty) => row.try_get::<Value, _>(i)?,
        _ => row.try_get_unchecked::<String, _>(i)?.into(),
    })
}
