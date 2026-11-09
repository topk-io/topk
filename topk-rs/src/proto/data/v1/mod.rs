include!(concat!(env!("OUT_DIR"), "/topk.data.v1.rs"));

use chrono::{DateTime, Utc};

use crate::Error;

mod data_ext;
mod query_ext;
pub use data_ext::{IntoListValues, IntoMatrixValues, IntoSparseValues};

impl Partition {
    pub fn created_at(&self) -> Result<DateTime<Utc>, Error> {
        DateTime::parse_from_rfc3339(&self.created_at)
            .map(|created_at| created_at.to_utc())
            .map_err(|e| {
                Error::MalformedResponse(format!(
                    "partition {} created_at {:?}: {e}",
                    self.name, self.created_at
                ))
            })
    }
}

impl DeleteDocumentsRequest {
    pub fn ids(ids: impl Into<Vec<String>>) -> Self {
        DeleteDocumentsRequest {
            ids: ids.into(),
            expr: None,
        }
    }

    pub fn filter(expr: impl Into<LogicalExpr>) -> Self {
        DeleteDocumentsRequest {
            ids: vec![],
            expr: Some(expr.into()),
        }
    }
}

impl From<Vec<String>> for DeleteDocumentsRequest {
    fn from(ids: Vec<String>) -> Self {
        Self::ids(ids)
    }
}

impl From<LogicalExpr> for DeleteDocumentsRequest {
    fn from(expr: LogicalExpr) -> Self {
        Self::filter(expr)
    }
}
