use super::logical::LogicalExpr;
use crate::data::{
    list::{List, Values},
    scalar::Scalar, vector::{F32SparseVector, SparseVector},
};
use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
};

#[pyclass]
#[derive(Debug, Clone)]
pub struct Null;

#[derive(Debug, Clone, FromPyObject)]
pub enum FlexibleExpr {
    Scalar(Scalar),
    Expr(LogicalExpr),
}

impl Into<LogicalExpr> for FlexibleExpr {
    fn into(self) -> LogicalExpr {
        match self {
            FlexibleExpr::Scalar(s) => LogicalExpr::Literal { value: s },
            FlexibleExpr::Expr(e) => e,
        }
    }
}



#[derive(Debug, Clone)]
pub enum Vectorish {
    List(List),
    SparseVector(SparseVector),
    FloatList(Vec<f32>),
}


impl<'py> FromPyObject<'py> for Vectorish {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(v) = obj.downcast::<List>() {
            Ok(Vectorish::List(v.borrow().clone()))
        } else if let Ok(v) = obj.downcast::<SparseVector>() {
            Ok(Vectorish::SparseVector(v.get().clone()))
        } else if let Ok(v) = F32SparseVector::extract_bound(obj) {
            Ok(Vectorish::SparseVector(SparseVector::F32 {
                indices: v.indices,
                values: v.values,
            }))
        } else if let Ok(v) = obj.extract::<Vec<f32>>() {
            Ok(Vectorish::FloatList(v))
        } else {
            Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to Vector",
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
pub enum StringyWithList {
    Stringy(Stringy),
    StringList(Vec<String>),
}

impl Into<LogicalExpr> for StringyWithList {
    fn into(self) -> LogicalExpr {
        match self {
            StringyWithList::Stringy(s) => s.into(),
            StringyWithList::StringList(values) => LogicalExpr::Literal {
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
