use anyhow::{bail, ensure, Result};

const TABLES: &str = include_str!("tables.sql");
const DESCRIBE: &str = include_str!("describe.sql");
const INDEXES: &str = include_str!("indexes.sql");
const ACCESS_METHODS: &str = include_str!("access_methods.sql");
const TYPES: &str = include_str!("types.sql");
const SCHEMAS: &str = include_str!("schemas.sql");
const DATABASES: &str = include_str!("databases.sql");
const ROLES: &str = include_str!("roles.sql");
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
    if let ("d", Some(table)) = (name, arg) {
        return Ok(Some(Expansion {
            sql: fill(DESCRIBE, "table", table),
            describes: Some(table.to_owned()),
        }));
    }
    let query = match name {
        "d" | "dt" => TABLES,
        "di" => INDEXES,
        "dA" => ACCESS_METHODS,
        "dT" => TYPES,
        "dn" => SCHEMAS,
        "l" => DATABASES,
        "du" | "dg" => ROLES,
        "?" => HELP,
        _ => bail!("unknown command \\{name}; run \\? for the supported commands"),
    };
    Ok(Some(Expansion {
        sql: fill(query, "pattern", &like(arg.unwrap_or("*"))),
        describes: None,
    }))
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
