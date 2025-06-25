use crate::expr::{filter::FilterExpr, logical::LogicalExpr, select::SelectExpr};
use std::collections::HashMap;

#[derive(Clone)]
pub enum Stage {
    Select {
        exprs: HashMap<String, SelectExpr>,
    },
    Filter {
        expr: FilterExpr,
    },
    TopK {
        expr: LogicalExpr,
        k: u64,
        asc: bool,
    },
    Count {},
    Rerank {
        model: Option<String>,
        query: Option<String>,
        fields: Vec<String>,
        topk_multiple: Option<u32>,
    },
}

impl From<Stage> for crate::proto::v1::data::Stage {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::Select { exprs } => crate::proto::v1::data::Stage::select(exprs),
            Stage::Filter { expr } => crate::proto::v1::data::Stage::filter(expr),
            Stage::TopK { expr, k, asc } => {
                crate::proto::v1::data::Stage::topk(expr.into(), k, asc)
            }
            Stage::Count {} => crate::proto::v1::data::Stage::count(),
            Stage::Rerank {
                model,
                query,
                fields,
                topk_multiple,
            } => crate::proto::v1::data::Stage::rerank(model, query, fields, topk_multiple),
        }
    }
}
