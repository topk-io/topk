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

impl From<Stage> for topk_rs::query::Stage {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::Select { exprs } => topk_rs::query::Stage::Select {
                exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
            },
            Stage::Filter { expr } => topk_rs::query::Stage::Filter { expr: expr.into() },
            Stage::TopK { expr, k, asc } => topk_rs::query::Stage::TopK {
                expr: expr.into(),
                k: k.try_into().unwrap(),
                asc,
            },
            Stage::Count {} => topk_rs::query::Stage::Count {},
            Stage::Rerank {
                model,
                query,
                fields,
                topk_multiple,
            } => topk_rs::query::Stage::Rerank {
                model,
                query,
                fields,
                topk_multiple,
            },
        }
    }
}
