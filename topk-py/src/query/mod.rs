use crate::data::scalar::Scalar;
use crate::data::value::Value;
use crate::data::vector::Vector;
use crate::expr::filter::FilterExprUnion;
use crate::expr::flexible::Vectorish;
use crate::expr::function::FunctionExpr;
use crate::expr::logical::{LogicalExpr, UnaryOperator};
use crate::expr::select::SelectExprUnion;
use crate::expr::text::{Term, TextExpr};
use crate::module;
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

    m.add_wrapped(wrap_pyfunction!(select))?;
    m.add_wrapped(wrap_pyfunction!(filter))?;
    m.add_wrapped(wrap_pyfunction!(field))?;
    m.add_wrapped(wrap_pyfunction!(literal))?;
    m.add_wrapped(wrap_pyfunction!(r#match))?;
    m.add_wrapped(wrap_pyfunction!(not_))?;
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
pub fn vector_distance(field: String, query: Vectorish) -> FunctionExpr {
    FunctionExpr::VectorScore {
        field,
        query: match query {
            Vectorish::Raw(values) => Vector::F32(values),
            Vectorish::Value(value) => match value {
                Value::Vector(vector) => vector,
                _ => unreachable!("Invalid vector type: {:?}", value),
            },
        },
    }
}

#[pyfunction]
pub fn semantic_similarity(field: String, query: String) -> FunctionExpr {
    FunctionExpr::SemanticSimilarity { field, query }
}
