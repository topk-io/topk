use crate::proto::data::v1::LogicalExpr;

use super::Source;

impl Source {
    pub fn new(dataset: impl Into<String>) -> Self {
        Self {
            dataset: dataset.into(),
            filter: None,
        }
    }

    pub fn with_filter(mut self, filter: impl Into<LogicalExpr>) -> Self {
        self.filter = Some(filter.into());
        self
    }
}

impl From<String> for Source {
    fn from(dataset: String) -> Self {
        Source {
            dataset,
            filter: None,
        }
    }
}

impl From<&String> for Source {
    fn from(dataset: &String) -> Self {
        Self::from(dataset.clone())
    }
}

impl From<&str> for Source {
    fn from(dataset: &str) -> Self {
        Self::from(dataset.to_string())
    }
}
