use super::{logical_expr::LogicalExpression, text_expr::TextExpression};
use pyo3::prelude::*;
use topk_protos::v1::data;

#[pyclass]
#[derive(Debug, Clone)]
pub enum FilterExpression {
    Logical(LogicalExpression),
    Text(TextExpression),
}

impl Into<data::stage::filter_stage::FilterExpr> for FilterExpression {
    fn into(self) -> data::stage::filter_stage::FilterExpr {
        match self {
            FilterExpression::Logical(expr) => {
                data::stage::filter_stage::FilterExpr::logical(expr.into())
            }
            FilterExpression::Text(expr) => {
                data::stage::filter_stage::FilterExpr::text(expr.into())
            }
        }
    }
}

#[derive(Debug, Clone, FromPyObject)]
pub enum FilterExpressionUnion {
    #[pyo3(transparent)]
    Logical(LogicalExpression),

    #[pyo3(transparent)]
    Text(TextExpression),
}

impl Into<FilterExpression> for FilterExpressionUnion {
    fn into(self) -> FilterExpression {
        match self {
            FilterExpressionUnion::Logical(expr) => FilterExpression::Logical(expr),
            FilterExpressionUnion::Text(expr) => FilterExpression::Text(expr),
        }
    }
}
