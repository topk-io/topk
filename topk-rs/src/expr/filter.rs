use crate::expr::{logical::LogicalExpr, text::TextExpr};
use topk_protos::v1::data::stage::filter_stage::FilterExpr as FilterExprPb;

#[derive(Debug, Clone)]
pub enum FilterExpr {
    Logical(LogicalExpr),
    Text(TextExpr),
}

impl Into<FilterExprPb> for FilterExpr {
    fn into(self) -> FilterExprPb {
        match self {
            FilterExpr::Logical(expr) => FilterExprPb::logical(expr.into()),
            FilterExpr::Text(expr) => FilterExprPb::text(expr.into()),
        }
    }
}

impl From<LogicalExpr> for FilterExpr {
    fn from(expr: LogicalExpr) -> Self {
        FilterExpr::Logical(expr)
    }
}

impl From<TextExpr> for FilterExpr {
    fn from(expr: TextExpr) -> Self {
        FilterExpr::Text(expr)
    }
}
