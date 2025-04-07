use napi_derive::napi;
use std::collections::HashMap;

use super::{
    filter_expr::FilterExpressionUnion, logical_expr::LogicalExpression,
    select_expr::SelectExpression,
};

#[napi]
#[derive(Debug, Clone)]
pub enum Stage {
    Select {
        #[napi(ts_type = "Record<string, LogicalExpression | FunctionExpression>")]
        exprs: HashMap<String, SelectExpression>,
    },
    Filter {
        #[napi(ts_type = "LogicalExpression | TextExpression")]
        expr: FilterExpressionUnion,
    },
    TopK {
        expr: LogicalExpression,
        k: i32,
        asc: bool,
    },
    Count,
    Rerank {
        model: Option<String>,
        query: Option<String>,
        fields: Vec<String>,
        topk_multiple: Option<u32>,
    },
}
