use super::{
    expr_binary::BinaryOperator,
    expr_unary::UnaryOperator,
    flexible_expr::{Boolish, FlexibleExpr, Numeric},
    scalar::Scalar,
};
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum LogicalExpression {
    Null(),
    Field {
        name: String,
    },
    Literal {
        value: Scalar,
    },
    Unary {
        op: UnaryOperator,
        expr: Py<LogicalExpression>,
    },
    Binary {
        left: Py<LogicalExpression>,
        op: BinaryOperator,
        right: Py<LogicalExpression>,
    },
}

impl PartialEq for LogicalExpression {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LogicalExpression::Null(), LogicalExpression::Null()) => true,
            (LogicalExpression::Field { name: l }, LogicalExpression::Field { name: r }) => l == r,
            (LogicalExpression::Literal { value: l }, LogicalExpression::Literal { value: r }) => {
                l == r
            }
            (
                LogicalExpression::Unary {
                    op: l,
                    expr: l_expr,
                },
                LogicalExpression::Unary {
                    op: r,
                    expr: r_expr,
                },
            ) => l == r && l_expr.get() == r_expr.get(),
            (
                LogicalExpression::Binary {
                    left: l,
                    op: l_op,
                    right: l_right,
                },
                LogicalExpression::Binary {
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
impl LogicalExpression {
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }

    fn __add__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Add,
            right: Py::new(py, expr)?,
        })
    }

    fn __radd__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::Add,
            right: Py::new(py, self.clone())?,
        })
    }

    fn __sub__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Sub,
            right: Py::new(py, expr)?,
        })
    }

    fn __rsub__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::Sub,
            right: Py::new(py, self.clone())?,
        })
    }

    fn __mul__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Mul,
            right: Py::new(py, expr)?,
        })
    }

    fn __rmul__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::Mul,
            right: Py::new(py, self.clone())?,
        })
    }

    fn __div__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Div,
            right: Py::new(py, expr)?,
        })
    }

    fn __rdiv__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::Div,
            right: Py::new(py, self.clone())?,
        })
    }

    fn __truediv__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Div,
            right: Py::new(py, expr)?,
        })
    }

    fn __rtruediv__(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::Div,
            right: Py::new(py, self.clone())?,
        })
    }

    fn __and__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::And,
            right: Py::new(py, expr)?,
        })
    }

    fn __rand__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::And,
            right: Py::new(py, self.clone())?,
        })
    }

    fn __or__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Or,
            right: Py::new(py, expr)?,
        })
    }

    fn __ror__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, expr)?,
            op: BinaryOperator::Or,
            right: Py::new(py, self.clone())?,
        })
    }

    fn eq(&self, py: Python<'_>, other: FlexibleExpr) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Eq,
            right: Py::new(py, expr)?,
        })
    }

    fn ne(&self, py: Python<'_>, other: FlexibleExpr) -> PyResult<Self> {
        let expr: LogicalExpression = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::NotEq,
            right: Py::new(py, expr)?,
        })
    }

    fn __eq__(&self, other: &LogicalExpression) -> bool {
        self == other
    }
}

impl Into<topk_protos::v1::data::LogicalExpr> for LogicalExpression {
    fn into(self) -> topk_protos::v1::data::LogicalExpr {
        match self {
            LogicalExpression::Null() => unreachable!(),
            LogicalExpression::Field { name } => topk_protos::v1::data::LogicalExpr::field(name),
            LogicalExpression::Literal { value } => {
                topk_protos::v1::data::LogicalExpr::literal(value.into())
            }
            LogicalExpression::Unary { op, expr } => {
                topk_protos::v1::data::LogicalExpr::unary(op.into(), expr.get().clone().into())
            }
            LogicalExpression::Binary { left, op, right } => {
                topk_protos::v1::data::LogicalExpr::binary(
                    op.into(),
                    left.get().clone().into(),
                    right.get().clone().into(),
                )
            }
        }
    }
}
