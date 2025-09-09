use super::logical::LogicalExpr;
use crate::data::{
    list::{List, Values},
    scalar::Scalar,
};
use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyBool, PyFloat, PyInt, PyString},
};

#[derive(Debug, Clone)]
pub enum FlexibleExpr {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
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
pub enum Ordered {
    #[pyo3(transparent)]
    String(String),

    #[pyo3(transparent)]
    Numeric(Numeric),
}

impl Into<LogicalExpr> for Ordered {
    fn into(self) -> LogicalExpr {
        match self {
            Ordered::Numeric(n) => n.into(),
            Ordered::String(s) => LogicalExpr::Literal {
                value: Scalar::String(s),
            },
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
pub enum StringyWithList {
    Stringy(Stringy),
    List(Vec<String>),
}

impl Into<LogicalExpr> for StringyWithList {
    fn into(self) -> LogicalExpr {
        match self {
            StringyWithList::Stringy(s) => s.into(),
            StringyWithList::List(values) => LogicalExpr::Literal {
                value: Scalar::List(List {
                    values: Values::String(values),
                }),
            },
        }
    }
}

#[derive(Debug, Clone, FromPyObject)]
pub enum Iterable {
    #[pyo3(transparent)]
    String(String),

    #[pyo3(transparent)]
    List(List),

    #[pyo3(transparent)]
    StringList(Vec<String>),

    #[pyo3(transparent)]
    IntList(Vec<i64>),

    #[pyo3(transparent)]
    FloatList(Vec<f32>),

    #[pyo3(transparent)]
    Expr(LogicalExpr),
}

impl Into<LogicalExpr> for Iterable {
    fn into(self) -> LogicalExpr {
        match self {
            Iterable::String(s) => LogicalExpr::Literal {
                value: Scalar::String(s),
            },
            Iterable::List(l) => LogicalExpr::Literal {
                value: Scalar::List(l),
            },
            Iterable::StringList(l) => LogicalExpr::Literal {
                value: Scalar::List(List {
                    values: Values::String(l),
                }),
            },
            Iterable::IntList(l) => LogicalExpr::Literal {
                value: Scalar::List(List {
                    values: Values::I64(l),
                }),
            },
            Iterable::FloatList(l) => LogicalExpr::Literal {
                value: Scalar::List(List {
                    values: Values::F32(l),
                }),
            },
            Iterable::Expr(e) => e,
        }
    }
}
