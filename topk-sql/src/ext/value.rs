use sqlparser::ast::Value as SqlValue;

pub trait SqlValueExt {
    /// If this value is a `$N` placeholder, return its 0-based index.
    fn as_placeholder(&self) -> Option<usize>;
}

impl SqlValueExt for SqlValue {
    fn as_placeholder(&self) -> Option<usize> {
        match self {
            SqlValue::Placeholder(p) => p.strip_prefix('$')?.parse::<usize>().ok()?.checked_sub(1),
            _ => None,
        }
    }
}
