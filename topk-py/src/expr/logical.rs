use crate::data::scalar::Scalar;
use crate::expr::flexible::FlexibleExpr;
use crate::expr::flexible::{Boolish, Iterable, Numeric, Ordered, Stringy, StringyWithList};
use pyo3::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(eq, eq_int)]
pub enum UnaryOperator {
    Not,
    IsNull,
    IsNotNull,
    Abs,
    Ln,
    Exp,
    Sqrt,
    Square,
}

impl From<UnaryOperator> for topk_rs::proto::v1::data::logical_expr::unary_op::Op {
    fn from(op: UnaryOperator) -> Self {
        match op {
            UnaryOperator::Not => topk_rs::proto::v1::data::logical_expr::unary_op::Op::Not,
            UnaryOperator::IsNull => topk_rs::proto::v1::data::logical_expr::unary_op::Op::IsNull,
            UnaryOperator::IsNotNull => {
                topk_rs::proto::v1::data::logical_expr::unary_op::Op::IsNotNull
            }
            UnaryOperator::Abs => topk_rs::proto::v1::data::logical_expr::unary_op::Op::Abs,
            UnaryOperator::Ln => topk_rs::proto::v1::data::logical_expr::unary_op::Op::Ln,
            UnaryOperator::Exp => topk_rs::proto::v1::data::logical_expr::unary_op::Op::Exp,
            UnaryOperator::Sqrt => topk_rs::proto::v1::data::logical_expr::unary_op::Op::Sqrt,
            UnaryOperator::Square => topk_rs::proto::v1::data::logical_expr::unary_op::Op::Square,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(eq, eq_int)]
pub enum BinaryOperator {
    And,
    Or,
    Xor,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    StartsWith,
    Contains,
    In,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    MatchAll,
    MatchAny,
    Coalesce,
    Min,
    Max,
}

impl From<BinaryOperator> for topk_rs::proto::v1::data::logical_expr::binary_op::Op {
    fn from(op: BinaryOperator) -> Self {
        match op {
            BinaryOperator::And => topk_rs::proto::v1::data::logical_expr::binary_op::Op::And,
            BinaryOperator::Or => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Or,
            BinaryOperator::Eq => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Eq,
            BinaryOperator::NotEq => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Neq,
            BinaryOperator::Lt => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Lt,
            BinaryOperator::LtEq => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Lte,
            BinaryOperator::Gt => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Gt,
            BinaryOperator::GtEq => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Gte,
            BinaryOperator::StartsWith => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::StartsWith
            }
            BinaryOperator::Contains => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::Contains
            }
            BinaryOperator::In => topk_rs::proto::v1::data::logical_expr::binary_op::Op::In,
            BinaryOperator::MatchAll => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::MatchAll
            }
            BinaryOperator::MatchAny => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::MatchAny
            }
            BinaryOperator::Add => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Add,
            BinaryOperator::Sub => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Sub,
            BinaryOperator::Mul => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Mul,
            BinaryOperator::Div => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Div,
            BinaryOperator::Coalesce => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::Coalesce
            }
            BinaryOperator::Rem => unimplemented!("`rem` operator is not supported"),
            BinaryOperator::Xor => unimplemented!("`xor` operator is not supported"),
            BinaryOperator::Min => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Min,
            BinaryOperator::Max => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Max,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(eq, eq_int)]
pub enum TernaryOperator {
    Choose,
}

impl From<TernaryOperator> for topk_rs::proto::v1::data::logical_expr::ternary_op::Op {
    fn from(op: TernaryOperator) -> Self {
        match op {
            TernaryOperator::Choose => {
                topk_rs::proto::v1::data::logical_expr::ternary_op::Op::Choose
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(eq, eq_int)]
pub enum NaryOperator {
    All,
    Any,
}

impl From<NaryOperator> for topk_rs::proto::v1::data::logical_expr::nary_op::Op {
    fn from(op: NaryOperator) -> Self {
        match op {
            NaryOperator::All => topk_rs::proto::v1::data::logical_expr::nary_op::Op::All,
            NaryOperator::Any => topk_rs::proto::v1::data::logical_expr::nary_op::Op::Any,
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub enum LogicalExpr {
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
    Ternary {
        op: TernaryOperator,
        x: Py<LogicalExpr>,
        y: Py<LogicalExpr>,
        z: Py<LogicalExpr>,
    },
    Nary {
        op: NaryOperator,
        exprs: Vec<Py<LogicalExpr>>,
    },
}

impl std::fmt::Debug for LogicalExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
            Self::Ternary { op, x, y, z } => {
                write!(
                    f,
                    "Ternary(op={:?}, x={:?}, y={:?}, z={:?})",
                    op,
                    x.get(),
                    y.get(),
                    z.get()
                )
            }
            Self::Nary { op, exprs } => {
                write!(f, "Nary(op={:?}, exprs={:?})", op, exprs)
            }
        }
    }
}

impl PartialEq for LogicalExpr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LogicalExpr::Field { name: l }, LogicalExpr::Field { name: r }) => l == r,
            (LogicalExpr::Literal { value: l }, LogicalExpr::Literal { value: r }) => l == r,
            (
                LogicalExpr::Unary {
                    op: l_op,
                    expr: l_expr,
                },
                LogicalExpr::Unary {
                    op: r_op,
                    expr: r_expr,
                },
            ) => l_op == r_op && l_expr.get() == r_expr.get(),
            (
                LogicalExpr::Binary {
                    left: l_left,
                    op: l_op,
                    right: l_right,
                },
                LogicalExpr::Binary {
                    left: r_left,
                    op: r_op,
                    right: r_right,
                },
            ) => l_op == r_op && l_left.get() == r_left.get() && l_right.get() == r_right.get(),
            (
                LogicalExpr::Ternary {
                    op: l_op,
                    x: l_x,
                    y: l_y,
                    z: l_z,
                },
                LogicalExpr::Ternary {
                    op: r_op,
                    x: r_x,
                    y: r_y,
                    z: r_z,
                },
            ) => {
                l_op == r_op
                    && l_x.get() == r_x.get()
                    && l_y.get() == r_y.get()
                    && l_z.get() == r_z.get()
            }
            (
                LogicalExpr::Nary {
                    op: l_op,
                    exprs: l_exprs,
                },
                LogicalExpr::Nary {
                    op: r_op,
                    exprs: r_exprs,
                },
            ) => {
                l_op == r_op
                    && l_exprs.len() == r_exprs.len()
                    && l_exprs
                        .iter()
                        .zip(r_exprs.iter())
                        .all(|(l, r)| l.get() == r.get())
            }
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

    fn is_null(&self, py: Python<'_>) -> PyResult<Self> {
        Ok(Self::Unary {
            op: UnaryOperator::IsNull,
            expr: Py::new(py, self.clone())?,
        })
    }

    fn is_not_null(&self, py: Python<'_>) -> PyResult<Self> {
        Ok(Self::Unary {
            op: UnaryOperator::IsNotNull,
            expr: Py::new(py, self.clone())?,
        })
    }

    fn abs(&self, py: Python<'_>) -> PyResult<Self> {
        Ok(Self::Unary {
            op: UnaryOperator::Abs,
            expr: Py::new(py, self.clone())?,
        })
    }

    fn __abs__(&self, py: Python<'_>) -> PyResult<Self> {
        self.abs(py)
    }

    fn ln(&self, py: Python<'_>) -> PyResult<Self> {
        Ok(Self::Unary {
            op: UnaryOperator::Ln,
            expr: Py::new(py, self.clone())?,
        })
    }

    fn exp(&self, py: Python<'_>) -> PyResult<Self> {
        Ok(Self::Unary {
            op: UnaryOperator::Exp,
            expr: Py::new(py, self.clone())?,
        })
    }

    fn sqrt(&self, py: Python<'_>) -> PyResult<Self> {
        Ok(Self::Unary {
            op: UnaryOperator::Sqrt,
            expr: Py::new(py, self.clone())?,
        })
    }

    fn square(&self, py: Python<'_>) -> PyResult<Self> {
        Ok(Self::Unary {
            op: UnaryOperator::Square,
            expr: Py::new(py, self.clone())?,
        })
    }

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

    fn lt(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Lt,
            right: Py::new(py, expr)?,
        })
    }

    fn __lt__(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        self.lt(py, other)
    }

    fn __rlt__(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        self.gt(py, other)
    }

    fn lte(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::LtEq,
            right: Py::new(py, expr)?,
        })
    }

    fn __le__(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        self.lte(py, other)
    }

    fn __rle__(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        self.gte(py, other)
    }

    fn gt(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Gt,
            right: Py::new(py, expr)?,
        })
    }

    fn __gt__(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        self.gt(py, other)
    }

    fn __rgt__(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        self.lt(py, other)
    }

    fn gte(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::GtEq,
            right: Py::new(py, expr)?,
        })
    }

    fn __ge__(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        self.gte(py, other)
    }

    fn __rge__(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        self.lte(py, other)
    }

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

    fn and_(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::And,
            right: Py::new(py, expr)?,
        })
    }

    fn __and__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        self.and_(py, other)
    }

    fn __rand__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        self.and_(py, other)
    }

    fn or_(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        let expr: LogicalExpr = other.into();

        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Or,
            right: Py::new(py, expr)?,
        })
    }

    fn __or__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        self.or_(py, other)
    }

    fn __ror__(&self, py: Python<'_>, other: Boolish) -> PyResult<Self> {
        self.or_(py, other)
    }

    fn starts_with(&self, py: Python<'_>, other: Stringy) -> PyResult<Self> {
        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::StartsWith,
            right: Py::new(py, Into::<LogicalExpr>::into(other))?,
        })
    }

    fn contains(&self, py: Python<'_>, other: FlexibleExpr) -> PyResult<Self> {
        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Contains,
            right: Py::new(py, Into::<LogicalExpr>::into(other))?,
        })
    }

    fn in_(&self, py: Python<'_>, other: Iterable) -> PyResult<Self> {
        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::In,
            right: Py::new(py, Into::<LogicalExpr>::into(other))?,
        })
    }

    fn match_all(&self, py: Python<'_>, other: StringyWithList) -> PyResult<Self> {
        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::MatchAll,
            right: Py::new(py, Into::<LogicalExpr>::into(other))?,
        })
    }

    fn match_any(&self, py: Python<'_>, other: StringyWithList) -> PyResult<Self> {
        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::MatchAny,
            right: Py::new(py, Into::<LogicalExpr>::into(other))?,
        })
    }

    fn coalesce(&self, py: Python<'_>, other: Numeric) -> PyResult<Self> {
        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Coalesce,
            right: Py::new(py, Into::<LogicalExpr>::into(other))?,
        })
    }

    // Ternary operators

    fn choose(&self, py: Python<'_>, x: FlexibleExpr, y: FlexibleExpr) -> PyResult<Self> {
        Ok(Self::Ternary {
            op: TernaryOperator::Choose,
            x: Py::new(py, self.clone())?,
            y: Py::new(py, Into::<LogicalExpr>::into(x))?,
            z: Py::new(py, Into::<LogicalExpr>::into(y))?,
        })
    }

    /// Multiplies the scoring expression by the provided `boost` value if the `condition` is true.
    /// Otherwise, the scoring expression is unchanged (multiplied by 1).
    fn boost(&self, py: Python<'_>, condition: FlexibleExpr, boost: Numeric) -> PyResult<Self> {
        let condition_expr = Into::<LogicalExpr>::into(condition);
        let choose_expr =
            condition_expr.choose(py, FlexibleExpr::Expr(boost.into()), FlexibleExpr::Int(1))?;
        let choose_numeric = Numeric::Expr(choose_expr);
        self.mul(py, choose_numeric)
    }

    fn min(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Min,
            right: Py::new(py, Into::<LogicalExpr>::into(other))?,
        })
    }

    fn max(&self, py: Python<'_>, other: Ordered) -> PyResult<Self> {
        Ok(Self::Binary {
            left: Py::new(py, self.clone())?,
            op: BinaryOperator::Max,
            right: Py::new(py, Into::<LogicalExpr>::into(other))?,
        })
    }
}

impl From<LogicalExpr> for topk_rs::proto::v1::data::LogicalExpr {
    fn from(expr: LogicalExpr) -> Self {
        match expr {
            LogicalExpr::Field { name } => topk_rs::proto::v1::data::LogicalExpr::field(name),
            LogicalExpr::Literal { value } => topk_rs::proto::v1::data::LogicalExpr::literal(value),
            LogicalExpr::Unary { op, expr } => {
                topk_rs::proto::v1::data::LogicalExpr::unary(op, expr.get().clone())
            }
            LogicalExpr::Binary { left, op, right } => {
                topk_rs::proto::v1::data::LogicalExpr::binary(
                    op,
                    left.get().clone(),
                    right.get().clone(),
                )
            }
            LogicalExpr::Ternary { op, x, y, z } => topk_rs::proto::v1::data::LogicalExpr::ternary(
                op,
                x.get().clone(),
                y.get().clone(),
                z.get().clone(),
            ),
            LogicalExpr::Nary { op, exprs } => topk_rs::proto::v1::data::LogicalExpr::nary(
                op,
                exprs.into_iter().map(|e| e.get().clone()),
            ),
        }
    }
}
