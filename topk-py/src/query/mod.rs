use crate::data::scalar::Scalar;
use crate::data::value::Value;
use crate::expr::filter::FilterExprUnion;
use crate::expr::flexible::Ordered;
use crate::expr::function::FunctionExpr;
use crate::expr::logical::{BinaryOperator, LogicalExpr, NaryOperator, UnaryOperator};
use crate::expr::select::SelectExprUnion;
use crate::expr::text::{Term, TextExpr};
use crate::module;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;

mod query;
pub use query::{ConsistencyLevel, Query};

mod stage;

////////////////////////////////////////////////////////////
/// Query
///
/// This module contains the query definition for the TopK SDK.
////////////////////////////////////////////////////////////

#[pymodule]
#[pyo3(name = "query")]
pub fn pymodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    module!(m, "fn", fn_pymodule)?;

    m.add_class::<Query>()?;
    m.add_class::<LogicalExpr>()?;
    m.add_class::<FunctionExpr>()?;

    m.add_wrapped(wrap_pyfunction!(select))?;
    m.add_wrapped(wrap_pyfunction!(filter))?;
    m.add_wrapped(wrap_pyfunction!(field))?;
    m.add_wrapped(wrap_pyfunction!(literal))?;
    m.add_wrapped(wrap_pyfunction!(r#match))?;
    m.add_wrapped(wrap_pyfunction!(not_))?;
    m.add_wrapped(wrap_pyfunction!(min))?;
    m.add_wrapped(wrap_pyfunction!(max))?;
    m.add_wrapped(wrap_pyfunction!(abs))?;
    m.add_wrapped(wrap_pyfunction!(all))?;
    m.add_wrapped(wrap_pyfunction!(any))?;

    Ok(())
}

#[pyfunction]
#[pyo3(signature = (*args, **kwargs))]
pub fn select(
    args: Vec<String>,
    kwargs: Option<HashMap<String, SelectExprUnion>>,
) -> PyResult<Query> {
    Ok(Query::new().select(args, kwargs)?)
}

#[pyfunction]
#[pyo3(signature = (expr))]
pub fn filter(expr: FilterExprUnion) -> PyResult<Query> {
    Ok(Query::new().filter(expr)?)
}

#[pyfunction]
pub fn field(name: String) -> LogicalExpr {
    LogicalExpr::Field { name }
}

#[pyfunction]
pub fn literal(value: Scalar) -> LogicalExpr {
    LogicalExpr::Literal { value }
}

#[pyfunction]
#[pyo3(signature = (token, field=None, weight=1.0, all=false))]
pub fn r#match(token: String, field: Option<String>, weight: f32, all: bool) -> TextExpr {
    TextExpr::Terms {
        all,
        terms: vec![Term {
            token,
            field,
            weight,
        }],
    }
}

#[pyfunction]
pub fn not_(py: Python<'_>, expr: LogicalExpr) -> PyResult<LogicalExpr> {
    Ok(LogicalExpr::Unary {
        op: UnaryOperator::Not,
        expr: Py::new(py, expr)?,
    })
}

#[pyfunction]
pub fn all(py: Python<'_>, exprs: Vec<LogicalExpr>) -> PyResult<LogicalExpr> {
    Ok(LogicalExpr::Nary {
        op: NaryOperator::All,
        exprs: exprs
            .into_iter()
            .map(|e| Py::new(py, e))
            .collect::<Result<Vec<Py<LogicalExpr>>, PyErr>>()?,
    })
}

#[pyfunction]
pub fn any(py: Python<'_>, exprs: Vec<LogicalExpr>) -> PyResult<LogicalExpr> {
    Ok(LogicalExpr::Nary {
        op: NaryOperator::Any,
        exprs: exprs
            .into_iter()
            .map(|e| Py::new(py, e))
            .collect::<Result<Vec<Py<LogicalExpr>>, PyErr>>()?,
    })
}

#[pyfunction]
pub fn min(py: Python<'_>, left: Ordered, right: Ordered) -> PyResult<LogicalExpr> {
    Ok(LogicalExpr::Binary {
        left: Py::new(py, Into::<LogicalExpr>::into(left))?,
        op: BinaryOperator::Min,
        right: Py::new(py, Into::<LogicalExpr>::into(right))?,
    })
}

#[pyfunction]
pub fn max(py: Python<'_>, left: Ordered, right: Ordered) -> PyResult<LogicalExpr> {
    Ok(LogicalExpr::Binary {
        left: Py::new(py, Into::<LogicalExpr>::into(left))?,
        op: BinaryOperator::Max,
        right: Py::new(py, Into::<LogicalExpr>::into(right))?,
    })
}

#[pyfunction]
pub fn abs(py: Python<'_>, expr: LogicalExpr) -> PyResult<LogicalExpr> {
    Ok(LogicalExpr::Unary {
        op: UnaryOperator::Abs,
        expr: Py::new(py, expr)?,
    })
}

#[pymodule]
#[pyo3(name = "fn")]
pub fn fn_pymodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(bm25_score))?;
    m.add_wrapped(wrap_pyfunction!(vector_distance))?;
    m.add_wrapped(wrap_pyfunction!(semantic_similarity))?;

    Ok(())
}

#[pyfunction]
pub fn bm25_score() -> FunctionExpr {
    FunctionExpr::KeywordScore {}
}

#[pyfunction]
#[pyo3(signature = (field, query, skip_refine=false))]
pub fn vector_distance(field: String, query: Value, skip_refine: bool) -> PyResult<FunctionExpr> {
    match query {
        Value::List(list) => Ok(FunctionExpr::VectorScore {
            field,
            query: Value::List(list),
            skip_refine,
        }),
        Value::SparseVector(vector) => Ok(FunctionExpr::VectorScore {
            field,
            query: Value::SparseVector(vector),
            skip_refine,
        }),
        _ => Err(PyValueError::new_err(
            "Vector query must be a vector or sparse vector",
        )),
    }
}

#[pyfunction]
pub fn semantic_similarity(field: String, query: String) -> FunctionExpr {
    FunctionExpr::SemanticSimilarity { field, query }
}
