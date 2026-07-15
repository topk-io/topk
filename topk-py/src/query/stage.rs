use crate::expr::filter::FilterExpr;
use crate::expr::logical::LogicalExpr;
use crate::expr::select::SelectExpr;
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass]
#[derive(Debug, Clone)]
pub enum Stage {
    Select { exprs: HashMap<String, SelectExpr> },
    Filter { expr: FilterExpr },
    Limit { k: u64 },
    Sort { expr: LogicalExpr, asc: bool },
    Count {},
}

impl From<Stage> for topk_rs::proto::v1::data::Stage {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::Select { exprs } => topk_rs::proto::v1::data::Stage::select(exprs),
            Stage::Filter { expr } => topk_rs::proto::v1::data::Stage::filter(expr),
            Stage::Limit { k } => topk_rs::proto::v1::data::Stage::limit(k),
            Stage::Sort { expr, asc } => topk_rs::proto::v1::data::Stage::sort((
                expr.into(),
                if asc {
                    topk_rs::proto::v1::data::stage::sort_stage::SortOrder::Asc
                } else {
                    topk_rs::proto::v1::data::stage::sort_stage::SortOrder::Desc
                },
            )),
            Stage::Count {} => topk_rs::proto::v1::data::Stage::count(),
        }
    }
}
