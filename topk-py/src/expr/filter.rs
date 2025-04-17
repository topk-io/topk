use super::{logical::LogicalExpr, text::TextExpr};
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum FilterExpr {
    Logical(LogicalExpr),
    Text(TextExpr),
}

impl Into<topk_rs::expr::filter::FilterExpr> for FilterExpr {
    fn into(self) -> topk_rs::expr::filter::FilterExpr {
        match self {
            FilterExpr::Logical(expr) => topk_rs::expr::filter::FilterExpr::Logical(expr.into()),
            FilterExpr::Text(expr) => topk_rs::expr::filter::FilterExpr::Text(expr.into()),
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

impl Into<FilterExpr> for FilterExprUnion {
    fn into(self) -> FilterExpr {
        match self {
            FilterExprUnion::Logical(expr) => FilterExpr::Logical(expr),
            FilterExprUnion::Text(expr) => FilterExpr::Text(expr),
        }
    }
}
