use crate::import::source::Format;

use super::lit;

/// A SELECT under construction. Every method is one clause; nothing here knows
/// what the rows are for.
pub(super) struct Select {
    relation: String,
    projection: String,
    filters: Vec<String>,
    order: Option<String>,
    limit: Option<u64>,
    offset: u64,
    pushdown: Option<String>,
}

impl Select {
    pub(super) fn new(relation: impl Into<String>) -> Self {
        Self {
            relation: relation.into(),
            projection: "*".to_string(),
            filters: Vec::new(),
            order: None,
            limit: None,
            offset: 0,
            pushdown: None,
        }
    }

    /// `schema.table` in the attached catalog, which `connect` makes the default.
    pub(super) fn table(name: &str) -> Self {
        Select::new(name.split('.').map(quoted).collect::<Vec<_>>().join("."))
    }

    /// One path or glob, or a list of files.
    pub(super) fn file(format: Format, paths: &[String]) -> Self {
        // Take the union of every file read, so a column present in only some of
        // them is kept, not dropped to the first file's schema.
        let opts = match format {
            Format::Csv => ", union_by_name=true, null_padding=true, parallel=false",
            Format::Json | Format::Parquet => ", union_by_name=true",
            Format::Arrow | Format::Avro => "",
        };
        let reader = match format {
            Format::Csv => "read_csv_auto",
            Format::Json => "read_json_auto",
            Format::Parquet => "read_parquet",
            Format::Arrow => "read_arrow",
            Format::Avro => "read_avro",
        };
        let quoted: Vec<_> = paths
            .iter()
            .map(|path| format!("'{}'", lit(path)))
            .collect();
        let files = match quoted.as_slice() {
            [one] => one.clone(),
            _ => format!("[{}]", quoted.join(", ")),
        };
        Self::new(format!("{reader}({files}{opts})"))
    }

    /// Text columns with one value for every row.
    pub(super) fn constants<'a>(mut self, columns: Vec<(&'a str, &'a str)>) -> Self {
        for (name, value) in columns {
            self.relation = format!(
                "(SELECT *, '{}' AS {} FROM {})",
                lit(value),
                quoted(name),
                self.relation
            );
        }
        self
    }

    /// Runs the query on the attached server instead of in duckdb.
    pub(super) fn pushdown(mut self, catalog: &str) -> Self {
        self.pushdown = Some(catalog.to_string());
        self
    }

    pub(super) fn columns<'a>(mut self, columns: impl IntoIterator<Item = &'a str>) -> Self {
        self.projection = columns
            .into_iter()
            .map(quoted)
            .collect::<Vec<_>>()
            .join(", ");
        self
    }

    /// Selects every column of the relation but these.
    pub(super) fn except<'a>(mut self, columns: impl IntoIterator<Item = &'a str>) -> Self {
        let names = columns
            .into_iter()
            .map(|column| format!("'{}'", lit(column)))
            .collect::<Vec<_>>();
        if !names.is_empty() {
            self.projection = format!("COLUMNS(lambda c: c NOT IN [{}])", names.join(", "));
        }
        self
    }

    /// Appends a predicate; every call ANDs another.
    pub(super) fn filter(mut self, filter: Option<&str>) -> Self {
        if let Some(filter) = filter {
            self.filters.push(format!("({filter})"));
        }
        self
    }

    pub(super) fn after(mut self, column: &str, value: Option<&str>) -> Self {
        if let Some(value) = value {
            self.filters
                .push(format!("{} > '{}'", quoted(column), lit(value)));
        }
        self
    }

    pub(super) fn order_by(mut self, column: &str) -> Self {
        self.order = Some(quoted(column));
        self
    }

    pub(super) fn limit(mut self, limit: Option<u64>) -> Self {
        self.limit = limit;
        self
    }

    pub(super) fn offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    pub(super) fn into_sql(self) -> String {
        let mut sql = format!("SELECT {} FROM {}", self.projection, self.relation);
        if !self.filters.is_empty() {
            sql.push_str(&format!(" WHERE {}", self.filters.join(" AND ")));
        }
        if let Some(order) = self.order {
            sql.push_str(&format!(" ORDER BY {order}"));
        }
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        if self.offset > 0 {
            sql.push_str(&format!(" OFFSET {}", self.offset));
        }
        match self.pushdown {
            Some(catalog) => format!("SELECT * FROM postgres_query('{catalog}', '{}')", lit(&sql)),
            None => sql,
        }
    }
}

pub(super) fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::{Format, Select};

    #[test]
    fn clauses_compose() {
        let sql = Select::table("src.docs")
            .columns(["id", "body"])
            .filter(Some("active"))
            .after("id", Some("1"))
            .order_by("id")
            .limit(Some(10))
            .into_sql();

        assert_eq!(
            sql,
            "SELECT \"id\", \"body\" FROM \"src\".\"docs\" \
             WHERE (active) AND \"id\" > '1' ORDER BY \"id\" LIMIT 10"
        );
    }

    #[test]
    fn excepted_columns_are_quoted_as_names() {
        let sql = Select::new("read_parquet('docs.parquet')")
            .except(["raw", "author's note"])
            .offset(4)
            .into_sql();

        assert_eq!(
            sql,
            "SELECT COLUMNS(lambda c: c NOT IN ['raw', 'author''s note']) \
             FROM read_parquet('docs.parquet') OFFSET 4"
        );
        assert_eq!(
            Select::file(Format::Parquet, &["a.parquet".into(), "b.parquet".into()])
                .except([])
                .into_sql(),
            "SELECT * FROM read_parquet(['a.parquet', 'b.parquet'], union_by_name=true)"
        );
    }
}
