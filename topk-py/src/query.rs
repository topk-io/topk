use crate::data::query::Query;
use crate::data::select_expr::SelectExpressionUnion;
use crate::{data, module};
use pyo3::prelude::*;
use std::collections::HashMap;

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
    m.add_wrapped(wrap_pyfunction!(field))?;
    m.add_wrapped(wrap_pyfunction!(literal))?;
    m.add_wrapped(wrap_pyfunction!(r#match))?;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (*args, **kwargs))]
pub fn select(
    args: Vec<String>,
    kwargs: Option<HashMap<String, SelectExpressionUnion>>,
) -> PyResult<Query> {
    Ok(Query::new().select(args, kwargs)?)
}

#[pyfunction]
pub fn field(name: String) -> data::logical_expr::LogicalExpression {
    data::logical_expr::LogicalExpression::Field { name }
}

#[pyfunction]
pub fn literal(value: data::scalar::Scalar) -> data::logical_expr::LogicalExpression {
    data::logical_expr::LogicalExpression::Literal { value }
}

#[pyfunction]
#[pyo3(text_signature = "match(field, token, weight=1.0)")]
pub fn r#match(field: String, token: String, weight: f32) -> data::text_expr::TextExpression {
    data::text_expr::TextExpression::Terms {
        all: false,
        terms: vec![data::text_expr::Term {
            token,
            field: Some(field),
            weight,
        }],
    }
}

#[pymodule]
#[pyo3(name = "fn")]
pub fn fn_pymodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(keyword_score))?;
    m.add_wrapped(wrap_pyfunction!(vector_distance))?;

    Ok(())
}

#[pyfunction]
pub fn keyword_score() -> data::function_expr::FunctionExpression {
    data::function_expr::FunctionExpression::KeywordScore {}
}

#[pyfunction]
pub fn vector_distance(field: String, query: Vec<f32>) -> data::function_expr::FunctionExpression {
    data::function_expr::FunctionExpression::VectorScore { field, query }
}
