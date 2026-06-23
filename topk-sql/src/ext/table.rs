use sqlparser::ast::{ObjectName, TableFactor};

pub trait ObjectNameExt {
    // The schema qualifier of the name (the part before the table), if any:
    // `schema.table` -> Some("schema"), `table` -> None.
    fn schema(&self) -> Option<&str>;
}

impl ObjectNameExt for ObjectName {
    fn schema(&self) -> Option<&str> {
        let (_table, qualifiers) = self.0.split_last()?;
        qualifiers.last()?.as_ident().map(|i| i.value.as_str())
    }
}

pub trait TableFactorExt {
    fn table_name(&self) -> Option<&ObjectName>;
}

impl TableFactorExt for TableFactor {
    fn table_name(&self) -> Option<&ObjectName> {
        match self {
            TableFactor::Table { name, .. } => Some(name),
            _ => None,
        }
    }
}
