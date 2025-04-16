use std::collections::HashMap;

use super::{
    filter_expr::FilterExpressionUnion, logical_expr::LogicalExpression,
    select_expr::SelectExpression,
};

#[derive(Debug, Clone)]
pub enum Stage {
    Select {
        exprs: HashMap<String, SelectExpression>,
    },
    Filter {
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
