use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use clap::ArgGroup;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{ContentArrangement, Table};
use futures::StreamExt;
use serde_json::{Map, Value};
use sqlx::postgres::{PgRow, PgTypeInfo};
use sqlx::{Column, Either, Postgres, Row, Type, ValueRef};
use tokio::time::timeout;

use crate::client::SqlClient;
use crate::config::Config;
use crate::endpoint::DataEndpoint;
use crate::output::{json_line, Output};
use crate::pager::Pager;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Text output prints a table per this many rows instead of holding the whole result.
const TABLE_ROWS: usize = 50;
const MIN_WIDTH: u16 = 20;

#[derive(clap::Args)]
#[command(group(ArgGroup::new("input").required(true).args(["query", "file"])))]
pub struct SqlArgs {
    /// Statements to run, separated by `;`
    pub query: Option<String>,

    /// Read the statements from a file, or `-` for stdin
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Maximum seconds without a SQL result; 0 disables the timeout (excludes paging)
    #[arg(long, value_name = "SECONDS", default_value_t = 60)]
    pub idle_timeout: u64,

    #[command(flatten)]
    pub data: DataEndpoint,
}

impl SqlArgs {
    fn input_sql(&self) -> Result<String> {
        let sql = match (&self.query, &self.file) {
            (Some(query), _) => query.clone(),
            (None, Some(path)) if path.as_os_str() == "-" => {
                let mut sql = String::new();
                std::io::stdin()
                    .read_to_string(&mut sql)
                    .context("reading stdin")?;
                sql
            }
            (None, Some(path)) => std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?,
            (None, None) => bail!("pass the SQL as an argument or with --file"),
        };
        ensure!(!sql.trim().is_empty(), "no SQL to run");
        Ok(sql)
    }
}

pub async fn run(config: Config, args: &SqlArgs, output: Output) -> Result<ExitCode> {
    let sql = args.input_sql()?;
    let mut client = timeout(CONNECT_TIMEOUT, SqlClient::connect(&config, &args.data))
        .await
        .with_context(|| format!("connecting timed out after {CONNECT_TIMEOUT:?}"))??;
    // Text is for people and may page; JSON is for programs and never does.
    let mut out = match output {
        Output::Text => Pager::stdout(),
        Output::Json => Pager::direct(),
    };
    let result = async {
        let mut printer = Printer::new(BufWriter::new(&mut out), output);
        let mut results = client.execute(&sql);
        // Each statement's rows print as they arrive; an error stops the rest.
        loop {
            let item = match args.idle_timeout {
                0 => results.next().await,
                seconds => timeout(Duration::from_secs(seconds), results.next())
                    .await
                    .with_context(|| format!("no SQL result for {seconds}s (--idle-timeout)"))?,
            };
            let Some(item) = item else { break };
            match item? {
                Either::Right(row) => printer.row(row)?,
                Either::Left(done) => printer.done(done.rows_affected())?,
            }
        }
        Ok(())
    }
    .await;
    // Even after an error, so the terminal returns only once the pager is closed.
    out.finish().context("running the pager")?;
    match result {
        Err(e) if !Pager::quit(&e) => Err(e),
        _ => Ok(ExitCode::SUCCESS),
    }
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
        ty if is::<bool>(ty) => row.try_get::<bool, _>(i)?.into(),
        ty if is::<i16>(ty) => row.try_get::<i16, _>(i)?.into(),
        ty if is::<i32>(ty) => row.try_get::<i32, _>(i)?.into(),
        ty if is::<i64>(ty) => row.try_get::<i64, _>(i)?.into(),
        ty if is::<f32>(ty) => row.try_get::<f32, _>(i)?.into(),
        ty if is::<f64>(ty) => row.try_get::<f64, _>(i)?.into(),
        ty if is::<Value>(ty) => row.try_get::<Value, _>(i)?,
        _ => row.try_get_unchecked::<String, _>(i)?.into(),
    })
}

/// Whether a column of this type decodes as `T`, the check `try_get` itself makes.
fn is<T: Type<Postgres>>(ty: &PgTypeInfo) -> bool {
    T::compatible(ty)
}
