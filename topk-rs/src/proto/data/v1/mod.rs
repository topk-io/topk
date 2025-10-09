include!(concat!(env!("OUT_DIR"), "/topk.data.v1.rs"));

mod data_ext;
mod query_ext;
pub use data_ext::IntoListValues;

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
