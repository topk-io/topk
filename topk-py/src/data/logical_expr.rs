use super::{
    expr_binary::BinaryOperator,
    expr_unary::UnaryOperator,
    flexible_expr::{Boolish, FlexibleExpr, Numeric, Stringy},
    scalar::Scalar,
};
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub enum LogicalExpr {
    Null(),
    Field {
        name: String,
    },
    Literal {
        value: Scalar,
    },
    Unary {
        op: UnaryOperator,
        expr: Py<LogicalExpr>,
    },
    Binary {
        left: Py<LogicalExpr>,
        op: BinaryOperator,
        right: Py<LogicalExpr>,
    },
}

impl std::fmt::Debug for LogicalExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null() => write!(f, "Null"),
            Self::Field { name } => write!(f, "field({})", name),
            Self::Literal { value } => write!(f, "literal({:?})", value),
            Self::Unary { op, expr } => {
                write!(f, "Unary(op={:?}, expr={:?})", op, expr.get())
            }
            Self::Binary { left, op, right } => {
                write!(
                    f,
                    "Binary(left={:?}, op={:?}, right={:?})",
                    left.get(),
                    op,
                    right.get()
                )
            }
        }
    }
}

impl PartialEq for LogicalExpr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LogicalExpr::Null(), LogicalExpr::Null()) => true,
            (LogicalExpr::Field { name: l }, LogicalExpr::Field { name: r }) => l == r,
            (LogicalExpr::Literal { value: l }, LogicalExpr::Literal { value: r }) => l == r,
            (
                LogicalExpr::Unary {
                    op: l,
                    expr: l_expr,
                },
                LogicalExpr::Unary {
                    op: r,
                    expr: r_expr,
                },
            ) => l == r && l_expr.get() == r_expr.get(),
            (
                LogicalExpr::Binary {
                    left: l,
                    op: l_op,
                    right: l_right,
                },
                LogicalExpr::Binary {
                    left: r,
                    op: r_op,
                    right: r_right,
                },
            ) => l.get() == r.get() && l_op == r_op && l_right.get() == r_right.get(),
            _ => false,
        }
    }
}

#[pymethods]
impl LogicalExpr {
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }

    fn _expr_eq(&self, other: &LogicalExpr) -> bool {
        self == other
    }

    // Comparison operators

    fn eq(&self, py: Python<'_>, other: FlexibleExpr) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Eq,
            right: Py::new(py, expr)?,
        })
    }

    fn __eq__(&self, py: Python<'_>, other: FlexibleExpr) -> PyResult<Self> {
        self.eq(py, other)
    }

    fn ne(&self, py: Python<'_>, other: FlexibleExpr) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::NotEq,
            right: Py::new(py, expr)?,
        })
    }

    fn __ne__(&self, py: Python<'_>, other: FlexibleExpr) -> PyResult<Self> {
        self.ne(py, other)
    }

    fn lt(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Lt,
            right: Py::new(py, expr)?,
        })
    }

    fn __lt__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.lt(py, other)
    }

    fn __rlt__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.gt(py, other)
    }

    fn lt_eq(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::LtEq,
            right: Py::new(py, expr)?,
        })
    }

    fn __le__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.lt_eq(py, other)
    }

    fn __rle__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.gt_eq(py, other)
    }

    fn gt(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Gt,
            right: Py::new(py, expr)?,
        })
    }

    fn __gt__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.gt(py, other)
    }

    fn __rgt__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.lt(py, other)
    }

    fn gt_eq(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::GtEq,
            right: Py::new(py, expr)?,
        })
    }

    fn __ge__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.gt_eq(py, other)
    }

    fn __rge__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.lt_eq(py, other)
    }

    // Arithmetic operators

    fn add(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Add,
            right: Py::new(py, expr)?,
        })
    }

    fn __add__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.add(py, other)
    }

    fn __radd__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.add(py, other)
    }

    fn sub(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Sub,
            right: Py::new(py, expr)?,
        })
    }

    fn __sub__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.sub(py, other)
    }

    fn __rsub__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::Sub,
            right: Py::new(py, self.clone())?,
        })
    }

    fn mul(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Mul,
            right: Py::new(py, expr)?,
        })
    }

    fn __mul__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.mul(py, other)
    }

    fn __rmul__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.mul(py, other)
    }

    fn div(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Div,
            right: Py::new(py, expr)?,
        })
    }

    fn __div__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.div(py, other)
    }

    fn __truediv__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        self.div(py, other)
    }

    fn __rdiv__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::Div,
            right: Py::new(py, self.clone())?,
        })
    }

    fn __rtruediv__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::Div,
            right: Py::new(py, self.clone())?,
        })
    }

    // Boolean operators

    fn and(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::And,
            right: Py::new(py, expr)?,
        })
    }

    fn __and__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        self.and(py, other)
    }

    fn __rand__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        self.and(py, other)
    }

    fn or(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Or,
            right: Py::new(py, expr)?,
        })
    }

    fn __or__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        self.or(py, other)
    }

    fn __ror__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        self.or(py, other)
    }

    // String operators

    fn starts_with(&self, py: Python<'_>, other: Stringy) -> PyResult<Self> {
        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::StartsWith,
            right: Py::new(py, Into::<LogicalExpr>::into(other))?,
        })
    }
}

impl Into<topk_rs::data::logical_expr::LogicalExpr> for LogicalExpr {
    fn into(self) -> topk_rs::data::logical_expr::LogicalExpr {
        match self {
            LogicalExpr::Null() => topk_rs::data::logical_expr::LogicalExpr::Null(),
            LogicalExpr::Field { name } => topk_rs::data::logical_expr::LogicalExpr::Field { name },
            LogicalExpr::Literal { value } => topk_rs::data::logical_expr::LogicalExpr::Literal {
                value: value.into(),
            },
            LogicalExpr::Unary { op, expr } => topk_rs::data::logical_expr::LogicalExpr::Unary {
                op: op.into(),
                expr: Box::new(expr.get().clone().into()),
            },
            LogicalExpr::Binary { left, op, right } => {
                topk_rs::data::logical_expr::LogicalExpr::Binary {
                    left: Box::new(left.get().clone().into()),
                    op: op.into(),
                    right: Box::new(right.get().clone().into()),
                }
            }
        }
    }
}
