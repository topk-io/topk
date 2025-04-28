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

impl From<Stage> for topk_protos::v1::data::Stage {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::Select { exprs } => topk_protos::v1::data::Stage::select(exprs),
            Stage::Filter { expr } => topk_protos::v1::data::Stage::filter(expr),
            Stage::TopK { expr, k, asc } => topk_protos::v1::data::Stage::topk(expr.into(), k, asc),
            Stage::Count {} => topk_protos::v1::data::Stage::count(),
            Stage::Rerank {
                model,
                query,
                fields,
                topk_multiple,
            } => topk_protos::v1::data::Stage::rerank(model, query, fields, topk_multiple),
        }
    }
}
