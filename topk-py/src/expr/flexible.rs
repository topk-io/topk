use super::logical::LogicalExpr;
use crate::data::{
    scalar::Scalar,
    vector::{F32SparseVector, F32Vector, SparseVector, Vector},
};
use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyBool, PyFloat, PyInt, PyString},
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
    DenseU8(Vec<u8>),
    DenseF32(Vec<f32>),
    SparseU8(Vec<u32>, Vec<u8>),
    SparseF32(Vec<u32>, Vec<f32>),
}

impl<'py> FromPyObject<'py> for Vectorish {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(v) = obj.downcast::<Vector>() {
            match v.get().clone() {
                Vector::F32(values) => Ok(Vectorish::DenseF32(values)),
                Vector::U8(values) => Ok(Vectorish::DenseU8(values)),
            }
        } else if let Ok(v) = obj.downcast::<SparseVector>() {
            match v.get().clone() {
                SparseVector::F32 { indices, values } => Ok(Vectorish::SparseF32(indices, values)),
                SparseVector::U8 { indices, values } => Ok(Vectorish::SparseU8(indices, values)),
            }
        } else if let Ok(v) = F32Vector::extract_bound(obj) {
            Ok(Vectorish::DenseF32(v.values))
        } else if let Ok(v) = F32SparseVector::extract_bound(obj) {
            Ok(Vectorish::SparseF32(v.indices, v.values))
        } else {
            Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to Vectorish",
                obj.get_type().name()
            )))
        }
    }
}
