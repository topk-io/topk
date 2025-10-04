include!(concat!(env!("OUT_DIR"), "/topk.data.v1.rs"));

mod data_ext;
mod query_ext;
pub use data_ext::IntoListValues;

impl From<Vec<String>> for DeleteDocumentsRequest {
    fn from(ids: Vec<String>) -> Self {
        DeleteDocumentsRequest { ids, expr: None }
    }
}

impl From<LogicalExpr> for DeleteDocumentsRequest {
    fn from(expr: LogicalExpr) -> Self {
        DeleteDocumentsRequest {
            ids: vec![],
            expr: Some(expr),
        }
    }
}
