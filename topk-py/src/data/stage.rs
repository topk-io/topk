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
}

impl Into<topk_protos::v1::data::Stage> for Stage {
    fn into(self) -> topk_protos::v1::data::Stage {
        match self {
            Stage::Select { exprs } => topk_protos::v1::data::Stage::select(exprs),
            Stage::Filter { expr } => topk_protos::v1::data::Stage::filter(expr),
            Stage::TopK { expr, k, asc } => topk_protos::v1::data::Stage::topk(expr.into(), k, asc),
        }
    }
}
