use sqlparser::ast::{ObjectName, TableFactor};

pub trait ObjectNameExt {
    fn as_table_ref(&self) -> Option<(Option<&str>, &str)>;
}

impl ObjectNameExt for ObjectName {
    fn as_table_ref(&self) -> Option<(Option<&str>, &str)> {
        let table = self.0.last()?.as_ident()?.value.as_str();
        let schema = (self.0.len() > 1)
            .then(|| {
                self.0[self.0.len() - 2]
                    .as_ident()
                    .map(|i| i.value.as_str())
            })
            .flatten();
        Some((schema, table))
    }
}

pub trait TableFactorExt {
    fn as_table_ref(&self) -> Option<(Option<&str>, &str)>;
}

impl TableFactorExt for TableFactor {
    fn as_table_ref(&self) -> Option<(Option<&str>, &str)> {
        match self {
            TableFactor::Table { name, .. } => name.as_table_ref(),
            _ => None,
        }
    }
}
