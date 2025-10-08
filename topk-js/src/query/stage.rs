use crate::expr::{filter::FilterExpression, logical::LogicalExpression, select::SelectExpression};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Stage {
    Select {
        exprs: HashMap<String, SelectExpression>,
    },
    Filter {
        expr: FilterExpression,
    },
    Limit {
        k: i32,
    },
    Sort {
        expr: LogicalExpression,
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

impl From<Stage> for topk_rs::proto::v1::data::Stage {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::Select { exprs } => topk_rs::proto::v1::data::Stage::select(exprs),
            Stage::Filter { expr } => topk_rs::proto::v1::data::Stage::filter(expr),
            Stage::Limit { k } => topk_rs::proto::v1::data::Stage::limit(k.try_into().unwrap()),
            Stage::Sort { expr, asc } => topk_rs::proto::v1::data::Stage::sort(expr.into(), asc),
            Stage::Count {} => topk_rs::proto::v1::data::Stage::count(),
            Stage::Rerank {
                model,
                query,
                fields,
                topk_multiple,
            } => topk_rs::proto::v1::data::Stage::rerank(model, query, fields, topk_multiple),
        }
    }
}
