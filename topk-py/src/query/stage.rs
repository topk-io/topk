use crate::expr::filter::FilterExpr;
use crate::expr::logical::LogicalExpr;
use crate::expr::select::SelectExpr;
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass]
#[derive(Debug, Clone)]
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

impl Into<topk_rs::query::Stage> for Stage {
    fn into(self) -> topk_rs::query::Stage {
        match self {
            Stage::Select { exprs } => topk_rs::query::Stage::Select {
                exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
            },
            Stage::Filter { expr } => topk_rs::query::Stage::Filter { expr: expr.into() },
            Stage::TopK { expr, k, asc } => topk_rs::query::Stage::TopK {
                expr: expr.into(),
                k,
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
