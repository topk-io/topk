use super::logical::LogicalExpr;
use pyo3::{exceptions::PyValueError, prelude::*};

#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl FromPyObject<'_, '_> for SortOrder {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        match ob.extract::<String>()?.as_str() {
            "asc" => Ok(SortOrder::Asc),
            "desc" => Ok(SortOrder::Desc),
            other => Err(PyValueError::new_err(format!(
                "sort order must be \"asc\" or \"desc\", got {other:?}"
            ))),
        }
    }
}

impl From<bool> for SortOrder {
    fn from(asc: bool) -> Self {
        if asc { SortOrder::Asc } else { SortOrder::Desc }
    }
}

impl From<SortOrder> for topk_rs::proto::v1::data::stage::sort_stage::SortOrder {
    fn from(order: SortOrder) -> Self {
        match order {
            SortOrder::Asc => Self::Asc,
            SortOrder::Desc => Self::Desc,
        }
    }
}

/// An expression to sort by with its sort order, extracted from an `(expr, "asc" | "desc")` pair.
#[pyclass]
#[derive(Debug, Clone)]
pub struct SortExpr {
    pub expr: LogicalExpr,
    pub order: SortOrder,
}

#[derive(Debug, Clone, FromPyObject)]
pub enum SortExprsUnion {
    #[pyo3(transparent)]
    Single(LogicalExpr),

    #[pyo3(transparent)]
    Many(Vec<(LogicalExpr, SortOrder)>),
}
