use crate::expr::{
    aggregate::AggregateExpression, filter::FilterExpression, logical::LogicalExpression,
    select::SelectExpression, sort::SortExpression,
};
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
    Offset {
        offset: i32,
    },
    Sort {
        exprs: Vec<SortExpression>,
    },
    Count,
    GroupBy {
        keys: HashMap<String, LogicalExpression>,
        aggs: HashMap<String, AggregateExpression>,
    },
}

impl From<Stage> for topk_rs::proto::v1::data::Stage {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::Select { exprs } => topk_rs::proto::v1::data::Stage::select(exprs),
            Stage::Filter { expr } => topk_rs::proto::v1::data::Stage::filter(expr),
            Stage::Limit { k } => topk_rs::proto::v1::data::Stage::limit(k.try_into().unwrap()),
            Stage::Sort { exprs } => topk_rs::proto::v1::data::Stage::sort(
                exprs
                    .into_iter()
                    .map(|se| (se.expr.into(), se.order.into()))
                    .collect::<Vec<_>>(),
            ),
            Stage::Offset { offset } => {
                topk_rs::proto::v1::data::Stage::offset(offset.try_into().unwrap())
            }
            Stage::Count {} => topk_rs::proto::v1::data::Stage::count(),
            Stage::GroupBy { keys, aggs } => topk_rs::proto::v1::data::Stage::group_by(keys, aggs),
        }
    }
}
