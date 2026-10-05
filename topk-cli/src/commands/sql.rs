use std::io::{BufWriter, Write};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use clap::ArgGroup;
use clap_stdin::{FileOrStdin, MaybeStdin};
use futures::StreamExt;
use serde_json::{Map, Value};
use sqlx::postgres::PgRow;
use sqlx::{Column, Either, Postgres, Row, Type, ValueRef};
use tokio::time::timeout;

use crate::client::SqlClient;
use crate::config::Config;
use crate::endpoint::DataEndpoint;
use crate::meta;
use crate::output::{json_line, table, Output};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

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
    let (sql, mut describes) = match meta::expand(&sql)? {
        Some(meta) => (meta.sql, meta.describes),
        None => (sql, None),
    };
    let mut client = timeout(CONNECT_TIMEOUT, SqlClient::connect(&config, &args.data))
        .await
        .with_context(|| format!("connecting timed out after {CONNECT_TIMEOUT:?}"))??;
    let mut printer = Printer::new(BufWriter::new(std::io::stdout().lock()), output);
    let mut results = client.execute(&sql);
    while let Some(item) = results.next().await {
        match item? {
            Either::Right(row) => printer.row(row)?,
            Either::Left(done) => {
                // Describing a table that returns zero columns means the table does not exist.
                if let Some(table) = describes.take() {
                    ensure!(printer.rows > 0, "did not find a table named {table:?}");
                }
                printer.done(done.rows_affected())?
            }
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
            Output::Text => self.table.push(row),
        }
        Ok(())
    }

    fn done(&mut self, affected: u64) -> Result<()> {
        let rows = std::mem::take(&mut self.rows);
        if self.output == Output::Text {
            self.print_table()?;
            match (rows, affected) {
                (0, 0) => eprintln!("(0 rows)"),
                (0, 1) => eprintln!("OK, 1 row affected"),
                (0, n) => eprintln!("OK, {n} rows affected"),
                (1, _) => eprintln!("(1 row)"),
                (n, _) => eprintln!("({n} rows)"),
            }
        }
        Ok(self.out.flush()?)
    }

    fn print_table(&mut self) -> Result<()> {
        let Some(first) = self.table.first() else {
            return Ok(());
        };
        let mut rendered = table(first.columns().iter().map(|column| column.name()));
        for row in self.table.drain(..) {
            rendered.add_row(
                (0..row.len())
                    // As the server printed it; NULL is empty.
                    .map(|i| {
                        Ok(row
                            .try_get_unchecked::<Option<String>, _>(i)?
                            .unwrap_or_default())
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
        }
        writeln!(self.out, "{rendered}")?;
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
