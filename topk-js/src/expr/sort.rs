use super::logical::LogicalExpression;
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Sort order.
#[napi(string_enum = "lowercase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl From<bool> for SortOrder {
    fn from(asc: bool) -> Self {
        if asc { SortOrder::Asc } else { SortOrder::Desc }
    }
}

impl From<SortOrder> for topk_rs::proto::v1::data::stage::sort_stage::SortOrder {
    fn from(order: SortOrder) -> Self {
        match order {
            SortOrder::Asc => Self::Asc,
            SortOrder::Desc => Self::Desc,
        }
    }
}

/// An expression to sort by with its sort order.
#[napi(object, namespace = "query")]
pub struct SortExpr<'env> {
    /// The expression to sort by.
    pub expr: ClassInstance<'env, LogicalExpression>,
    /// Sort order.
    pub order: SortOrder,
}

#[derive(Debug, Clone)]
pub struct SortExpression {
    pub expr: LogicalExpression,
    pub order: SortOrder,
}

impl From<SortExpr<'_>> for SortExpression {
    fn from(se: SortExpr) -> Self {
        SortExpression {
            expr: (*se.expr).clone(),
            order: se.order,
        }
    }
}
