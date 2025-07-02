use super::{logical::LogicalExpr, text::TextExpr};
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum FilterExpr {
    Logical(LogicalExpr),
    Text(TextExpr),
}

impl From<FilterExpr> for topk_rs::proto::v1::data::stage::filter_stage::FilterExpr {
    fn from(expr: FilterExpr) -> Self {
        match expr {
            FilterExpr::Logical(expr) => {
                topk_rs::proto::v1::data::stage::filter_stage::FilterExpr::logical(expr)
            }
            FilterExpr::Text(expr) => {
                topk_rs::proto::v1::data::stage::filter_stage::FilterExpr::text(expr)
            }
        }
    }
}

#[derive(Debug, Clone, FromPyObject)]
pub enum FilterExprUnion {
    #[pyo3(transparent)]
    Logical(LogicalExpr),

    #[pyo3(transparent)]
    Text(TextExpr),
}

impl From<FilterExprUnion> for FilterExpr {
    fn from(expr: FilterExprUnion) -> Self {
        match expr {
            FilterExprUnion::Logical(expr) => FilterExpr::Logical(expr),
            FilterExprUnion::Text(expr) => FilterExpr::Text(expr),
        }
    }
}
