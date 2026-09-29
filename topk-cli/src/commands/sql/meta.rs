use anyhow::{bail, ensure, Result};

const TABLES: &str = include_str!("tables.sql");
const DESCRIBE: &str = include_str!("describe.sql");
const INDEXES: &str = include_str!("indexes.sql");
const HELP: &str = include_str!("help.sql");

#[derive(Debug, PartialEq)]
pub struct Expansion {
    pub sql: String,
    /// The table `\d TABLE` describes
    pub describes: Option<String>,
}

pub fn expand(input: &str) -> Result<Option<Expansion>> {
    let Some(command) = input.trim().strip_prefix('\\') else {
        // plain SQL
        return Ok(None);
    };
    let mut words = command.split_whitespace();
    let name = words.next().unwrap_or_default();
    let arg = words.next();
    ensure!(name != "?" || arg.is_none(), "\\? takes no arguments");
    ensure!(
        words.next().is_none(),
        "\\{name} takes at most one argument"
    );
    let describes = match (name, arg) {
        ("d", Some(table)) => Some(table.to_owned()),
        _ => None,
    };
    let sql = match (name, arg) {
        ("d", Some(table)) => fill(DESCRIBE, "table", table),
        ("d" | "dt", pattern) => fill(TABLES, "pattern", &like(pattern.unwrap_or("*"))),
        ("di", pattern) => fill(INDEXES, "pattern", &like(pattern.unwrap_or("*"))),
        ("?", None) => HELP.to_owned(),
        _ => bail!("unknown command \\{name}; run \\? for the supported commands"),
    };
    Ok(Some(Expansion { sql, describes }))
}

pub fn table(table: &str) -> String {
    fill(TABLES, "pattern", &like(table))
}

/// Converts psql-style wildcards (`*` and `?`) to SQL `LIKE` syntax for catalog queries.
/// All other characters match literally, including SQL wildcards `%` and `_`.
/// See <https://www.postgresql.org/docs/current/app-psql.html#APP-PSQL-PATTERNS>.
fn like(pattern: &str) -> String {
    let mut value = String::with_capacity(pattern.len());
    for c in pattern.chars() {
        match c {
            '*' => value.push('%'),
            '?' => value.push('_'),
            '\\' | '%' | '_' => {
                value.push('\\');
                value.push(c);
            }
            c => value.push(c),
        }
    }
    value
}

fn fill(query: &str, name: &str, value: &str) -> String {
    query.replace(
        &format!("{{{{{name}}}}}"),
        &format!("'{}'", value.replace('\'', "''")),
    )
}
