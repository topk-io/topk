use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum AggregateExpr {
    Count { field: Option<String> },
    Sum { field: String },
    Min { field: String },
    Max { field: String },
    Avg { field: String },
}

impl From<AggregateExpr> for topk_rs::proto::v1::data::AggregateExpr {
    fn from(expr: AggregateExpr) -> Self {
        match expr {
            AggregateExpr::Count { field } => {
                topk_rs::proto::v1::data::AggregateExpr::count(field)
            }
            AggregateExpr::Sum { field } => topk_rs::proto::v1::data::AggregateExpr::sum(field),
            AggregateExpr::Min { field } => topk_rs::proto::v1::data::AggregateExpr::min(field),
            AggregateExpr::Max { field } => topk_rs::proto::v1::data::AggregateExpr::max(field),
            AggregateExpr::Avg { field } => topk_rs::proto::v1::data::AggregateExpr::avg(field),
        }
    }
}
