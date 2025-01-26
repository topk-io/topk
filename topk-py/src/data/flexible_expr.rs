use super::{logical_expr::LogicalExpression, scalar::Scalar};
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
    Expr(LogicalExpression),
}

impl Into<LogicalExpression> for FlexibleExpr {
    fn into(self) -> LogicalExpression {
        match self {
            FlexibleExpr::String(s) => LogicalExpression::Literal {
                value: Scalar::String(s),
            },
            FlexibleExpr::Int(i) => LogicalExpression::Literal {
                value: Scalar::Int(i),
            },
            FlexibleExpr::Float(f) => LogicalExpression::Literal {
                value: Scalar::Float(f),
            },
            FlexibleExpr::Bool(b) => LogicalExpression::Literal {
                value: Scalar::Bool(b),
            },
            FlexibleExpr::Null(_) => LogicalExpression::Null(),
            FlexibleExpr::Expr(e) => e,
        }
    }
}

impl<'py> FromPyObject<'py> for FlexibleExpr {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let obj = ob.as_ref();

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
        } else if let Ok(e) = obj.downcast::<LogicalExpression>() {
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
    Expr(LogicalExpression),
}

impl Into<LogicalExpression> for Numeric {
    fn into(self) -> LogicalExpression {
        match self {
            Numeric::Int(i) => LogicalExpression::Literal {
                value: Scalar::Int(i),
            },
            Numeric::Float(f) => LogicalExpression::Literal {
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
    Expr(LogicalExpression),
}

impl Into<LogicalExpression> for Boolish {
    fn into(self) -> LogicalExpression {
        match self {
            Boolish::Bool(b) => LogicalExpression::Literal {
                value: Scalar::Bool(b),
            },
            Boolish::Expr(e) => e,
        }
    }
}
