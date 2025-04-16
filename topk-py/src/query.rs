use crate::data::function_expr::VectorQuery;
use crate::data::query::Query;
use crate::data::select_expr::SelectExprUnion;
use crate::{data, module};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyList;
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
    m.add_wrapped(wrap_pyfunction!(count))?;
    m.add_wrapped(wrap_pyfunction!(field))?;
    m.add_wrapped(wrap_pyfunction!(literal))?;
    m.add_wrapped(wrap_pyfunction!(r#match))?;
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
pub fn count() -> PyResult<Query> {
    Query::new().count()
}

#[pyfunction]
pub fn field(name: String) -> data::logical_expr::LogicalExpr {
    data::logical_expr::LogicalExpr::Field { name }
}

#[pyfunction]
pub fn literal(value: data::scalar::Scalar) -> data::logical_expr::LogicalExpr {
    data::logical_expr::LogicalExpr::Literal { value }
}

#[pyfunction]
#[pyo3(signature = (token, field=None, weight=1.0, all=false))]
pub fn r#match(
    token: String,
    field: Option<String>,
    weight: f32,
    all: bool,
) -> data::text_expr::TextExpr {
    data::text_expr::TextExpr::Terms {
        all,
        terms: vec![data::text_expr::Term {
            token,
            field,
            weight,
        }],
    }
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
pub fn bm25_score() -> data::function_expr::FunctionExpr {
    data::function_expr::FunctionExpr::KeywordScore {}
}

#[derive(Debug, Clone)]
pub enum VectorQueryArg {
    // Float32 (raw) query vector
    F32(Vec<f32>),
    // U8 (binary or scalar quantized) query vector
    U8(Vec<u8>),
}

impl<'py> FromPyObject<'py> for VectorQueryArg {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let obj = ob.as_ref();

        match obj.downcast::<PyList>() {
            Ok(list) => {
                // Try converting to vector from starting with most restrictive type first.
                if let Ok(values) = list.extract::<Vec<u8>>() {
                    Ok(VectorQueryArg::U8(values))
                } else if let Ok(values) = list.extract::<Vec<f32>>() {
                    Ok(VectorQueryArg::F32(values))
                } else {
                    Err(PyTypeError::new_err(format!(
                        "Can't convert from {:?} to VectorQuery",
                        obj.get_type().name()
                    )))
                }
            }
            _ => Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to VectorQuery",
                obj.get_type().name()
            ))),
        }
    }
}

#[pyfunction]
pub fn vector_distance(field: String, query: VectorQueryArg) -> data::function_expr::FunctionExpr {
    data::function_expr::FunctionExpr::VectorScore {
        field,
        query: match query {
            VectorQueryArg::F32(values) => VectorQuery::F32(values),
            VectorQueryArg::U8(values) => VectorQuery::U8(values),
        },
    }
}

#[pyfunction]
pub fn semantic_similarity(field: String, query: String) -> data::function_expr::FunctionExpr {
    data::function_expr::FunctionExpr::SemanticSimilarity { field, query }
}
