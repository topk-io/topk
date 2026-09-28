use std::io::Write;

use clap::Parser;
use serde_json::{json, Value};
use tempfile::NamedTempFile;
use test_context::{test_context, AsyncTestContext};
use uuid::Uuid;

use topk::commands::sql::SqlArgs;

#[path = "../common/command.rs"]
mod command;
use command::TestCommand;

/// `topk` with the environment's API key, region and host.
fn ok(args: &[&str], stdin: Option<&str>) -> String {
    command(args, stdin).ok()
}

fn fails(args: &[&str], stdin: Option<&str>) -> String {
    command(args, stdin).fails()
}

fn command(args: &[&str], stdin: Option<&str>) -> TestCommand {
    let command = TestCommand::new(args);
    match stdin {
        Some(input) => command.stdin(input),
        None => command,
    }
}

fn json_lines(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// A collection of books, dropped after the test.
struct Books {
    table: String,
}

impl AsyncTestContext for Books {
    async fn setup() -> Self {
        let table = format!("cli_sql_{}", &Uuid::new_v4().simple().to_string()[..8]);
        ok(
            &[
                "sql",
                &format!("CREATE TABLE {table} (title TEXT NOT NULL INDEX keyword_index(), rating FLOAT)"),
            ],
            None,
        );
        ok(
            &[
                "sql",
                &format!(
                    "INSERT INTO {table} (_id, title, rating) \
                     VALUES ('1', 'Dune', 4.5), ('2', 'Emma', NULL)"
                ),
            ],
            None,
        );
        Self { table }
    }

    async fn teardown(self) {
        ok(
            &["sql", &format!("DROP TABLE IF EXISTS {}", self.table)],
            None,
        );
    }
}

impl Books {
    fn select(&self) -> String {
        format!(
            "SELECT _id, title, rating::float8 FROM {} WITH (consistency = 'strong') \
             ORDER BY _id LIMIT 10",
            self.table
        )
    }
}

#[test_context(Books)]
#[tokio::test]
async fn json_output_keeps_value_types(books: &mut Books) {
    assert_eq!(
        json_lines(&ok(&["-o", "json", "sql", &books.select()], None)),
        [
            json!({ "_id": "1", "title": "Dune", "rating": 4.5 }),
            json!({ "_id": "2", "title": "Emma", "rating": null }),
        ]
    );
}

#[test_context(Books)]
#[tokio::test]
async fn text_output_prints_a_table_per_statement(books: &mut Books) {
    let stdout = ok(
        &[
            "sql",
            &format!("{}; SELECT COUNT(*) FROM {}", books.select(), books.table),
        ],
        None,
    );
    assert!(stdout.contains("Dune"), "{stdout}");
    assert!(stdout.contains("(2 rows)"), "{stdout}");
    assert!(stdout.contains("_count"), "{stdout}");
    assert!(stdout.contains("(1 row)"), "{stdout}");
}

#[test_context(Books)]
#[tokio::test]
async fn reads_statements_from_file_or_stdin(books: &mut Books) {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", books.select()).unwrap();
    let from_file = ok(
        &["-o", "json", "sql", "-f", file.path().to_str().unwrap()],
        None,
    );
    let from_stdin = ok(&["-o", "json", "sql", "-f", "-"], Some(&books.select()));
    assert_eq!(json_lines(&from_file).len(), 2);
    assert_eq!(from_file, from_stdin);
}

#[test_context(Books)]
#[tokio::test]
async fn failed_statement_exits_non_zero(books: &mut Books) {
    let stderr = fails(&["sql", &format!("SELECT FROM {}", books.table)], None);
    assert!(stderr.contains("error"), "{stderr}");
}

#[test]
fn rejects_missing_or_conflicting_input() {
    assert!(fails(&["sql", "-f", "-"], Some("  \n")).contains("no SQL to run"));
    assert!(fails(&["sql"], None).contains("required"));
    assert!(fails(&["sql", "SELECT 1", "-f", "q.sql"], None).contains("cannot be used with"));
}

#[test_context(Books)]
#[tokio::test]
async fn large_results_print_a_table_per_fifty_rows(books: &mut Books) {
    let values = (0..120)
        .map(|i| format!("('n{i:03}', 'Book {i}', {i}.5)"))
        .collect::<Vec<_>>()
        .join(", ");
    ok(
        &[
            "sql",
            &format!(
                "INSERT INTO {} (_id, title, rating) VALUES {values}",
                books.table
            ),
        ],
        None,
    );
    let select = format!(
        "SELECT _id FROM {} WITH (consistency = 'strong') ORDER BY _id LIMIT 200",
        books.table
    );
    let stdout = ok(&["sql", &select], None);
    assert_eq!(stdout.matches("│ _id").count(), 3, "{stdout}");
    assert!(stdout.ends_with("(122 rows)\n"), "{stdout}");
    assert_eq!(
        json_lines(&ok(&["-o", "json", "sql", &select], None)).len(),
        122
    );
}

#[test_context(Books)]
#[tokio::test]
async fn empty_select_prints_zero_rows(books: &mut Books) {
    let select = format!(
        "SELECT _id FROM {} WHERE rating > 100 LIMIT 10",
        books.table
    );
    assert_eq!(ok(&["sql", &select], None), "(0 rows)\n");
    assert_eq!(ok(&["-o", "json", "sql", &select], None), "");
}

#[test]
fn idle_timeout_can_be_overridden_or_disabled() {
    #[derive(Parser)]
    struct Args {
        #[command(flatten)]
        sql: SqlArgs,
    }

    assert_eq!(
        Args::try_parse_from(["sql", "SELECT 1"])
            .unwrap()
            .sql
            .idle_timeout,
        60
    );
    for seconds in ["0", "300"] {
        let args = Args::try_parse_from(["sql", "SELECT 1", "--idle-timeout", seconds]).unwrap();
        assert_eq!(args.sql.idle_timeout, seconds.parse::<u64>().unwrap());
    }
    for seconds in ["-1", "invalid"] {
        assert!(Args::try_parse_from(["sql", "SELECT 1", "--idle-timeout", seconds]).is_err());
    }
}
