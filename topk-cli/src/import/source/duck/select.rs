use std::fmt::{self, Write};

use super::{lit, quoted};

pub(super) struct Plan {
    select: Select,
    pub(super) from: String,
    pub(super) filter: Option<String>,
    pub(super) position: Position,
}

pub(super) enum Position {
    Id(String),
    Offset(u64),
}

pub(super) struct Select {
    relation: String,
    from: String,
    projection: String,
    filters: Vec<String>,
    filter: Option<String>,
    order: Option<String>,
    limit: Option<u64>,
    offset: u64,
    output: Output,
}

enum Output {
    Duckdb,
    Postgres,
}

impl Select {
    pub(super) fn postgres(relation: String) -> Self {
        let mut select = Self::from(relation);
        select.output = Output::Postgres;
        select
    }

    /// Names the source shown in read errors when it differs from the relation.
    pub(super) fn reading(mut self, from: impl Into<String>) -> Self {
        self.from = from.into();
        self
    }

    pub(super) fn columns<'a>(mut self, columns: impl IntoIterator<Item = &'a str>) -> Self {
        self.projection = columns
            .into_iter()
            .map(|column| quoted(column, '"'))
            .collect::<Vec<_>>()
            .join(", ");
        self
    }

    /// Selects only requested columns present in the relation.
    pub(super) fn existing_columns<'a>(
        mut self,
        columns: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let names = columns
            .into_iter()
            .map(|column| format!("'{}'", lit(column)))
            .collect::<Vec<_>>()
            .join(", ");
        self.projection = format!("COLUMNS(c -> c IN [{names}])");
        self
    }

    pub(super) fn filter(mut self, filter: Option<String>) -> Self {
        if let Some(filter) = &filter {
            self.filters.push(format!("({filter})"));
        }
        self.filter = filter;
        self
    }

    pub(super) fn limit(mut self, limit: Option<u64>) -> Self {
        self.limit = limit;
        self
    }

    pub(super) fn by_id(mut self, id: &str, after: Option<&str>) -> Plan {
        let order = quoted(id, '"');
        if let Some(after) = after {
            self.filters.push(format!("{order} > '{}'", lit(after)));
        }
        self.order = Some(order);
        self.plan(Position::Id(id.to_string()))
    }

    pub(super) fn by_offset(mut self, offset: u64) -> Plan {
        self.offset = offset;
        self.plan(Position::Offset(offset))
    }

    fn plan(mut self, position: Position) -> Plan {
        let from = std::mem::take(&mut self.from);
        let filter = self.filter.take();
        Plan {
            select: self,
            from,
            filter,
            position,
        }
    }

    fn write_query(&self, out: &mut impl Write) -> fmt::Result {
        write!(out, "SELECT {} FROM {}", self.projection, self.relation)?;
        if !self.filters.is_empty() {
            write!(out, " WHERE {}", self.filters.join(" AND "))?;
        }
        if let Some(order) = &self.order {
            write!(out, " ORDER BY {order}")?;
        }
        if let Some(limit) = self.limit {
            write!(out, " LIMIT {limit}")?;
        }
        if self.offset > 0 {
            write!(out, " OFFSET {}", self.offset)?;
        }
        Ok(())
    }
}

impl From<String> for Select {
    fn from(relation: String) -> Self {
        Self {
            from: relation.clone(),
            relation,
            projection: "*".to_string(),
            filters: Vec::new(),
            filter: None,
            order: None,
            limit: None,
            offset: 0,
            output: Output::Duckdb,
        }
    }
}

impl fmt::Display for Select {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.output {
            Output::Duckdb => self.write_query(f),
            Output::Postgres => {
                let mut sql = String::new();
                self.write_query(&mut sql)?;
                write!(f, "SELECT * FROM postgres_query('src', '{}')", lit(&sql))
            }
        }
    }
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.select.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, Select};

    #[test]
    fn by_id() {
        let plan = Select::from("src.docs".to_string())
            .reading("docs")
            .columns(["id", "body"])
            .filter(Some("active".to_string()))
            .limit(Some(10))
            .by_id("id", Some("1"));

        assert_eq!(
            plan.to_string(),
            "SELECT \"id\", \"body\" FROM src.docs WHERE (active) AND \"id\" > '1' ORDER BY \"id\" LIMIT 10"
        );
        assert_eq!(plan.from, "docs");
        assert_eq!(plan.filter.as_deref(), Some("active"));
        assert!(matches!(plan.position, Position::Id(id) if id == "id"));
    }

    #[test]
    fn by_offset() {
        let plan = Select::from("read_parquet('docs.parquet')".to_string())
            .reading("docs.parquet")
            .existing_columns(["id", "author's note"])
            .by_offset(4);

        assert_eq!(
            plan.to_string(),
            "SELECT COLUMNS(c -> c IN ['id', 'author''s note']) FROM read_parquet('docs.parquet') OFFSET 4"
        );
        assert_eq!(plan.from, "docs.parquet");
        assert!(plan.filter.is_none());
        assert!(matches!(plan.position, Position::Offset(4)));
    }

    #[test]
    fn postgres() {
        let plan = Select::postgres("\"docs\"".to_string()).by_id("id", None);

        assert_eq!(
            plan.to_string(),
            "SELECT * FROM postgres_query('src', 'SELECT * FROM \"docs\" ORDER BY \"id\"')"
        );
    }
}
