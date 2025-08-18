use crate::proto::{
    data::v1::logical_expr::{self, binary_op, nary_op, ternary_op, unary_op, BinaryOp, UnaryOp},
    v1::data::{LogicalExpr, Value},
};

impl LogicalExpr {
    pub fn field(name: impl Into<String>) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::Field(name.into())),
        }
    }

    pub fn literal(value: impl Into<Value>) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::Literal(value.into())),
        }
    }

    #[inline(always)]
    pub fn unary(op: impl Into<unary_op::Op>, expr: impl Into<LogicalExpr>) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::UnaryOp(Box::new(
                logical_expr::UnaryOp {
                    op: op.into() as i32,
                    expr: Some(Box::new(expr.into())),
                },
            ))),
        }
    }

    pub fn not(expr: impl Into<LogicalExpr>) -> Self {
        LogicalExpr::unary(unary_op::Op::Not, expr)
    }

    pub fn is_null(self) -> Self {
        LogicalExpr::unary(unary_op::Op::IsNull, self)
    }

    pub fn is_not_null(self) -> Self {
        LogicalExpr::unary(unary_op::Op::IsNotNull, self)
    }

    pub fn abs(self) -> Self {
        LogicalExpr::unary(unary_op::Op::Abs, self)
    }

    pub fn ln(self) -> Self {
        LogicalExpr::unary(unary_op::Op::Ln, self)
    }

    pub fn exp(self) -> Self {
        LogicalExpr::unary(unary_op::Op::Exp, self)
    }

    pub fn sqrt(self) -> Self {
        LogicalExpr::unary(unary_op::Op::Sqrt, self)
    }

    pub fn square(self) -> Self {
        LogicalExpr::unary(unary_op::Op::Square, self)
    }

    #[inline(always)]
    pub fn binary(
        op: impl Into<binary_op::Op>,
        left: impl Into<LogicalExpr>,
        right: impl Into<LogicalExpr>,
    ) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: op.into() as i32,
                    left: Some(Box::new(left.into())),
                    right: Some(Box::new(right.into())),
                },
            ))),
        }
    }

    pub fn and(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::And, self, right)
    }

    pub fn or(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Or, self, right)
    }

    pub fn eq(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Eq, self, right)
    }

    pub fn neq(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Neq, self, right)
    }

    pub fn lt(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Lt, self, right)
    }

    pub fn lte(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Lte, self, right)
    }

    pub fn gt(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Gt, self, right)
    }

    pub fn gte(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Gte, self, right)
    }

    pub fn starts_with(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::StartsWith, self, right)
    }

    pub fn contains(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Contains, self, right)
    }

    /// If right is a list, evaluates to true if self is equal to one of the elements of right.
    /// If right is a string, evaluates to true if self is a substring of right.
    /// Equivalent to right.contains(self).
    pub fn in_(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::In, self, right)
    }

    /// Matches all terms against the field with keyword index.
    pub fn match_all(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::MatchAll, self, right)
    }

    /// Matches any term against the field with keyword index.
    pub fn match_any(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::MatchAny, self, right)
    }

    /// Coalesce nulls in the left expression with the provided value.
    pub fn coalesce(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Coalesce, self, right)
    }

    pub fn add(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Add, self, right)
    }

    pub fn mul(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Mul, self, right)
    }

    pub fn div(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Div, self, right)
    }

    pub fn sub(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Sub, self, right)
    }

    // Ternary operators

    #[inline(always)]
    pub fn ternary(
        op: impl Into<ternary_op::Op>,
        x: impl Into<LogicalExpr>,
        y: impl Into<LogicalExpr>,
        z: impl Into<LogicalExpr>,
    ) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::TernaryOp(Box::new(
                logical_expr::TernaryOp {
                    op: op.into() as i32,
                    x: Some(Box::new(x.into())),
                    y: Some(Box::new(y.into())),
                    z: Some(Box::new(z.into())),
                },
            ))),
        }
    }

    /// Condition operator that returns `x` if `self` is true, otherwise `y`.
    /// This operator can only be applied to boolean expressions.
    /// Arguments `x` and `y` must be of the same type or return types that
    /// can be converted to the same type.
    pub fn choose(self, x: impl Into<LogicalExpr>, y: impl Into<LogicalExpr>) -> Self {
        Self::ternary(ternary_op::Op::Choose, self, x, y)
    }

    /// Multiplies the scoring expression by the provided `boost` value if the `condition` is true.
    /// Otherwise, the scoring expression is unchanged (multiplied by 1).
    pub fn boost(self, condition: impl Into<LogicalExpr>, boost: impl Into<Value>) -> Self {
        self.mul(condition.into().choose(boost.into(), 1))
    }

    pub fn min(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Min, self, right)
    }

    pub fn max(self, right: impl Into<LogicalExpr>) -> Self {
        Self::binary(binary_op::Op::Max, self, right)
    }

    pub fn nary(
        op: impl Into<nary_op::Op>,
        exprs: impl IntoIterator<Item = impl Into<LogicalExpr>>,
    ) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::NaryOp(logical_expr::NaryOp {
                op: op.into() as i32,
                exprs: exprs.into_iter().map(|e| e.into()).collect(),
            })),
        }
    }

    pub fn all(exprs: impl IntoIterator<Item = impl Into<LogicalExpr>>) -> Self {
        Self::nary(nary_op::Op::All, exprs)
    }

    pub fn any(exprs: impl IntoIterator<Item = impl Into<LogicalExpr>>) -> Self {
        Self::nary(nary_op::Op::Any, exprs)
    }
}

impl From<Value> for LogicalExpr {
    fn from(value: Value) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<f32> for LogicalExpr {
    fn from(value: f32) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<f64> for LogicalExpr {
    fn from(value: f64) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<i32> for LogicalExpr {
    fn from(value: i32) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<u32> for LogicalExpr {
    fn from(value: u32) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<i64> for LogicalExpr {
    fn from(value: i64) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<u64> for LogicalExpr {
    fn from(value: u64) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<bool> for LogicalExpr {
    fn from(value: bool) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<String> for LogicalExpr {
    fn from(value: String) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<&str> for LogicalExpr {
    fn from(value: &str) -> Self {
        LogicalExpr::literal(value.to_string())
    }
}

impl From<Vec<String>> for LogicalExpr {
    fn from(value: Vec<String>) -> Self {
        LogicalExpr::literal(value)
    }
}

impl From<Vec<&str>> for LogicalExpr {
    fn from(value: Vec<&str>) -> Self {
        LogicalExpr::literal(
            value
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
    }
}

impl UnaryOp {
    pub fn not(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::Not as i32,
            expr: Some(Box::new(expr)),
        }
    }

    pub fn is_null(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::IsNull as i32,
            expr: Some(Box::new(expr)),
        }
    }

    pub fn is_not_null(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::IsNotNull as i32,
            expr: Some(Box::new(expr)),
        }
    }

    pub fn abs(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::Abs as i32,
            expr: Some(Box::new(expr)),
        }
    }

    pub fn ln(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::Ln as i32,
            expr: Some(Box::new(expr)),
        }
    }

    pub fn exp(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::Exp as i32,
            expr: Some(Box::new(expr)),
        }
    }

    pub fn sqrt(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::Sqrt as i32,
            expr: Some(Box::new(expr)),
        }
    }

    pub fn square(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::Square as i32,
            expr: Some(Box::new(expr)),
        }
    }
}

// Arithmetic operator overloads

impl<T: Into<LogicalExpr>> std::ops::Add<T> for LogicalExpr {
    type Output = LogicalExpr;

    fn add(self, rhs: T) -> Self::Output {
        LogicalExpr::add(self, rhs)
    }
}

impl<T: Into<LogicalExpr>> std::ops::Sub<T> for LogicalExpr {
    type Output = LogicalExpr;

    fn sub(self, rhs: T) -> Self::Output {
        LogicalExpr::sub(self, rhs)
    }
}

impl<T: Into<LogicalExpr>> std::ops::Mul<T> for LogicalExpr {
    type Output = LogicalExpr;

    fn mul(self, rhs: T) -> Self::Output {
        LogicalExpr::mul(self, rhs)
    }
}

impl<T: Into<LogicalExpr>> std::ops::Div<T> for LogicalExpr {
    type Output = LogicalExpr;

    fn div(self, rhs: T) -> Self::Output {
        LogicalExpr::div(self, rhs)
    }
}

// Logical operator overloads

impl<T: Into<LogicalExpr>> std::ops::BitAnd<T> for LogicalExpr {
    type Output = LogicalExpr;

    fn bitand(self, rhs: T) -> Self::Output {
        LogicalExpr::and(self, rhs)
    }
}

impl<T: Into<LogicalExpr>> std::ops::BitOr<T> for LogicalExpr {
    type Output = LogicalExpr;

    fn bitor(self, rhs: T) -> Self::Output {
        LogicalExpr::or(self, rhs)
    }
}

impl std::ops::Neg for LogicalExpr {
    type Output = LogicalExpr;

    fn neg(self) -> Self::Output {
        LogicalExpr::not(self)
    }
}

impl BinaryOp {
    pub fn and(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::And as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn or(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Or as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn eq(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Eq as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn neq(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Neq as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn lt(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Lt as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn lte(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Lte as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn gt(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Gt as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn gte(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Gte as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn min(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Min as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn max(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Max as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}
