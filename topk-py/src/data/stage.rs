use super::{
    filter_expr::FilterExpression, logical_expr::LogicalExpression, select_expr::SelectExpression,
};
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass]
#[derive(Debug, Clone)]
pub enum Stage {
    Select {
        exprs: HashMap<String, SelectExpression>,
    },
    Filter {
        expr: FilterExpression,
    },
    TopK {
        expr: LogicalExpression,
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

impl Into<topk_protos::v1::data::Stage> for Stage {
    fn into(self) -> topk_protos::v1::data::Stage {
        match self {
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
