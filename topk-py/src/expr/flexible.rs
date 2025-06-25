use std::collections::HashMap;

use super::logical::LogicalExpr;
use crate::data::{scalar::Scalar, value::Value};
use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyNone, PyString},
};

#[pyclass]
#[derive(Debug, Clone)]
pub struct Null;

#[derive(Debug, Clone)]
pub enum FlexibleExpr {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null(Null),
    Expr(LogicalExpr),
}

impl Into<LogicalExpr> for FlexibleExpr {
    fn into(self) -> LogicalExpr {
        match self {
            FlexibleExpr::String(s) => LogicalExpr::Literal {
                value: Scalar::String(s),
            },
            FlexibleExpr::Int(i) => LogicalExpr::Literal {
                value: Scalar::Int(i),
            },
            FlexibleExpr::Float(f) => LogicalExpr::Literal {
                value: Scalar::Float(f),
            },
            FlexibleExpr::Bool(b) => LogicalExpr::Literal {
                value: Scalar::Bool(b),
            },
            FlexibleExpr::Null(_) => LogicalExpr::Null(),
            FlexibleExpr::Expr(e) => e,
        }
    }
}

impl<'py> FromPyObject<'py> for FlexibleExpr {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(s) = obj.downcast_exact::<PyString>() {
            Ok(FlexibleExpr::String(s.extract()?))
        } else if let Ok(i) = obj.downcast_exact::<PyInt>() {
            Ok(FlexibleExpr::Int(i.extract()?))
        } else if let Ok(f) = obj.downcast_exact::<PyFloat>() {
            Ok(FlexibleExpr::Float(f.extract()?))
        } else if let Ok(b) = obj.downcast_exact::<PyBool>() {
            Ok(FlexibleExpr::Bool(b.extract()?))
        } else if let Ok(_) = obj.downcast_exact::<PyNone>() {
            Ok(FlexibleExpr::Null(Null))
        // NOTE: it's safe to use `downcast` for `LogicalExpr` since it's a custom type
        } else if let Ok(e) = obj.downcast::<LogicalExpr>() {
            Ok(FlexibleExpr::Expr(e.get().clone()))
        } else {
            Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to FlexibleExpr",
                obj.get_type().name()
            )))
        }
    }
}

#[derive(Debug, Clone, FromPyObject)]
pub enum Numeric {
    #[pyo3(transparent)]
    Int(i64),

    #[pyo3(transparent)]
    Float(f64),

    #[pyo3(transparent)]
    Expr(LogicalExpr),
}

impl Into<LogicalExpr> for Numeric {
    fn into(self) -> LogicalExpr {
        match self {
            Numeric::Int(i) => LogicalExpr::Literal {
                value: Scalar::Int(i),
            },
            Numeric::Float(f) => LogicalExpr::Literal {
                value: Scalar::Float(f),
            },
            Numeric::Expr(e) => e,
        }
    }
}

#[derive(Debug, Clone, FromPyObject)]
pub enum Boolish {
    #[pyo3(transparent)]
    Bool(bool),

    #[pyo3(transparent)]
    Expr(LogicalExpr),
}

impl Into<LogicalExpr> for Boolish {
    fn into(self) -> LogicalExpr {
        match self {
            Boolish::Bool(b) => LogicalExpr::Literal {
                value: Scalar::Bool(b),
            },
            Boolish::Expr(e) => e,
        }
    }
}

#[derive(Debug, Clone, FromPyObject)]
pub enum Stringy {
    #[pyo3(transparent)]
    String(String),

    #[pyo3(transparent)]
    Expr(LogicalExpr),
}

impl Into<LogicalExpr> for Stringy {
    fn into(self) -> LogicalExpr {
        match self {
            Stringy::String(s) => LogicalExpr::Literal {
                value: Scalar::String(s),
            },
            Stringy::Expr(e) => e,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Vectorish {
    DenseF32(Vec<f32>),
    SparseF32(Vec<u32>, Vec<f32>),
    SparseU8(Vec<u32>, Vec<u8>),
    Value(Value),
}

impl<'py> FromPyObject<'py> for Vectorish {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(v) = obj.downcast_exact::<PyList>() {
            Ok(Vectorish::DenseF32(v.extract()?))
        } else if let Ok(v) = obj.downcast_exact::<PyDict>() {
            let items = v.items();

            if items.len() == 0 {
                return Ok(Vectorish::SparseF32(vec![], vec![]));
            }

            if let Ok(mut items) = v.items().extract::<Vec<(u32, f32)>>() {
                items.sort_by_key(|(key, _)| *key);

                return Ok(Vectorish::SparseF32(
                    items.iter().map(|(i, _)| *i).collect(),
                    items.iter().map(|(_, v)| *v).collect(),
                ));
            }

            if let Ok(mut items) = v.items().extract::<Vec<(u32, u8)>>() {
                items.sort_by_key(|(key, _)| *key);

                return Ok(Vectorish::SparseU8(
                    items.iter().map(|(i, _)| *i).collect(),
                    items.iter().map(|(_, v)| *v).collect(),
                ));
            }

            Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to Value",
                obj.get_type().name()
            )))
        } else if let Ok(v) = obj.downcast::<Value>() {
            Ok(Vectorish::Value(v.get().clone()))
        } else {
            Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to Vectorish",
                obj.get_type().name()
            )))
        }
    }
}
