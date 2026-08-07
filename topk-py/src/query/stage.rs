use crate::expr::aggregate::AggregateExpr;
use crate::expr::filter::FilterExpr;
use crate::expr::logical::LogicalExpr;
use crate::expr::select::SelectExpr;
use crate::expr::sort::SortExpr;
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass]
#[derive(Debug, Clone)]
pub enum Stage {
    Select { exprs: HashMap<String, SelectExpr> },
    Filter { expr: FilterExpr },
    Limit { k: u64 },
    Offset { offset: u64 },
    Sort { exprs: Vec<SortExpr> },
    Count {},
    GroupBy {
        keys: HashMap<String, LogicalExpr>,
        aggs: HashMap<String, AggregateExpr>,
    },
}

impl From<Stage> for topk_rs::proto::v1::data::Stage {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::Select { exprs } => topk_rs::proto::v1::data::Stage::select(exprs),
            Stage::Filter { expr } => topk_rs::proto::v1::data::Stage::filter(expr),
            Stage::Limit { k } => topk_rs::proto::v1::data::Stage::limit(k),
            Stage::Sort { exprs } => topk_rs::proto::v1::data::Stage::sort(
                exprs
                    .into_iter()
                    .map(|se| (se.expr.into(), se.order.into()))
                    .collect::<Vec<_>>(),
            ),
            Stage::Offset { offset } => topk_rs::proto::v1::data::Stage::offset(offset),
            Stage::Count {} => topk_rs::proto::v1::data::Stage::count(),
            Stage::GroupBy { keys, aggs } => {
                topk_rs::proto::v1::data::Stage::group_by(keys, aggs)
            }
        }
    }
}
