use super::logical::LogicalExpr;
use crate::data::{scalar::Scalar, value::Value};
use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyBool, PyFloat, PyInt, PyNone, PyString},
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
        if let Ok(s) = obj.downcast::<PyString>() {
            Ok(FlexibleExpr::String(s.extract()?))
        } else if let Ok(i) = obj.downcast::<PyInt>() {
            Ok(FlexibleExpr::Int(i.extract()?))
        } else if let Ok(f) = obj.downcast::<PyFloat>() {
            Ok(FlexibleExpr::Float(f.extract()?))
        } else if let Ok(b) = obj.downcast::<PyBool>() {
            Ok(FlexibleExpr::Bool(b.extract()?))
        } else if let Ok(_) = obj.downcast::<PyNone>() {
            Ok(FlexibleExpr::Null(Null))
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

#[derive(Debug, Clone, FromPyObject)]
pub enum Vectorish {
    #[pyo3(transparent)]
    Raw(Vec<f32>),

    #[pyo3(transparent)]
    Value(Value),
}
