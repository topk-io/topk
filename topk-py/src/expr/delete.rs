use pyo3::prelude::*;

use super::logical::LogicalExpr;

#[derive(Debug, Clone, FromPyObject)]
pub enum DeleteExprUnion {
    #[pyo3(transparent)]
    Filter(LogicalExpr),

    #[pyo3(transparent)]
    Ids(Vec<String>),
}

impl From<DeleteExprUnion> for topk_rs::proto::v1::data::DeleteDocumentsRequest {
    fn from(expr: DeleteExprUnion) -> Self {
        match expr {
            DeleteExprUnion::Filter(expr) => {
                topk_rs::proto::v1::data::DeleteDocumentsRequest::filter(expr)
            }
            DeleteExprUnion::Ids(ids) => topk_rs::proto::v1::data::DeleteDocumentsRequest::ids(ids),
        }
    }
}
