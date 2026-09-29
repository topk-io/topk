use std::io::Write;

use serde_json::json;
use tempfile::NamedTempFile;
use test_context::{test_context, AsyncTestContext};
use uuid::Uuid;

#[path = "../common/command.rs"]
mod command;
mod meta;
use command::TestCommand;

struct Books {
    table: String,
}

impl AsyncTestContext for Books {
    async fn setup() -> Self {
        let table = format!("cli_sql_{}", &Uuid::new_v4().simple().to_string()[..8]);
        TestCommand::new()
            .args([
                "sql",
                &format!("CREATE TABLE {table} (title TEXT NOT NULL INDEX keyword_index(), rating FLOAT)"),
            ])
            .ok();
        TestCommand::new()
            .args([
                "sql",
                &format!(
                    "INSERT INTO {table} (_id, title, rating) \
                     VALUES ('1', 'Dune', 4.5), ('2', 'Emma', NULL)"
                ),
            ])
            .ok();
        Self { table }
    }

    async fn teardown(self) {
        TestCommand::new()
            .args(["sql", &format!("DROP TABLE IF EXISTS {}", self.table)])
            .ok();
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
        TestCommand::new()
            .args(["-o", "json", "sql", &books.select()])
            .json(),
        [
            json!({ "_id": "1", "title": "Dune", "rating": 4.5 }),
            json!({ "_id": "2", "title": "Emma", "rating": null }),
        ]
    );
}

#[test_context(Books)]
#[tokio::test]
async fn text_output_prints_a_table_per_statement(books: &mut Books) {
    let stdout = TestCommand::new()
        .args([
            "sql",
            &format!("{}; SELECT COUNT(*) FROM {}", books.select(), books.table),
        ])
        .ok();
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
    let json = |args: &[&str], stdin: Option<&str>| {
        TestCommand::new()
            .args(["-o", "json", "sql"])
            .args(args)
            .stdin(stdin)
            .json()
    };
    let from_file = json(&["-f", file.path().to_str().unwrap()], None);
    assert_eq!(from_file.len(), 2);
    assert_eq!(json(&["-f", "-"], Some(&books.select())), from_file);
    assert_eq!(json(&["-"], Some(&books.select())), from_file);
}

#[test_context(Books)]
#[tokio::test]
async fn failed_statement_exits_non_zero(books: &mut Books) {
    let stderr = TestCommand::new()
        .args(["sql", &format!("SELECT FROM {}", books.table)])
        .fails();
    assert!(stderr.contains("error"), "{stderr}");
}

#[test]
fn rejects_missing_or_conflicting_input() {
    let fails =
        |args: &[&str], stdin: Option<&str>| TestCommand::new().args(args).stdin(stdin).fails();
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
    TestCommand::new()
        .args([
            "sql",
            &format!(
                "INSERT INTO {} (_id, title, rating) VALUES {values}",
                books.table
            ),
        ])
        .ok();
    let select = format!(
        "SELECT _id FROM {} WITH (consistency = 'strong') ORDER BY _id LIMIT 200",
        books.table
    );
    let stdout = TestCommand::new().args(["sql", &select]).ok();
    assert_eq!(stdout.matches("│ _id").count(), 3, "{stdout}");
    assert!(stdout.ends_with("(122 rows)\n"), "{stdout}");
    assert_eq!(
        TestCommand::new()
            .args(["-o", "json", "sql", &select])
            .json()
            .len(),
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
    assert_eq!(TestCommand::new().args(["sql", &select]).ok(), "(0 rows)\n");
    assert_eq!(
        TestCommand::new().args(["-o", "json", "sql", &select]).ok(),
        ""
    );
}

#[test_context(Books)]
#[tokio::test]
async fn meta_commands_list_and_describe_tables(books: &mut Books) {
    let sql = |command: String| TestCommand::new().args(["sql", &command]).ok();
    let prefix = &books.table[..books.table.len() - 2];
    assert!(sql(format!("\\dt {prefix}*")).contains(&books.table));
    assert_eq!(
        TestCommand::new()
            .args(["-o", "json", "sql", &format!("\\dt {}", books.table)])
            .json(),
        [json!({ "name": books.table })]
    );
    let described = sql(format!("\\d {}", books.table));
    for expected in ["title", "rating", "double precision", "keyword_index"] {
        assert!(described.contains(expected), "{described}");
    }
    let indexes = sql(format!("\\di {}*", books.table));
    assert!(indexes.contains("keyword_index"), "{indexes}");
    assert!(sql("\\?".into()).contains("\\di [PATTERN]"));
    assert!(sql("\\dA *vector*".into()).contains("multi_vector_index"));
    assert!(sql("\\dT f32*".into()).contains("f32_vector"));
    assert!(sql("\\dn".into()).contains("public"));
    assert!(sql("\\l".into()).contains("default"));
    assert!(sql("\\du".into()).contains("postgres"));
    assert!(TestCommand::new()
        .args(["sql", "\\x"])
        .fails()
        .contains("unknown command \\x; run \\?"));
}
