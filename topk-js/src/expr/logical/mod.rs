mod binary_op;
mod boolish;
mod comparable;
mod flexible;
mod nary_op;
mod numeric;
mod ordered;
mod stringy;
mod ternary_op;
mod unary_op;

pub use binary_op::BinaryOperator;
pub use nary_op::NaryOp;
pub use numeric::Numeric;
pub use ordered::Ordered;
pub use ternary_op::TernaryOperator;
pub use unary_op::UnaryOperator;

use crate::{
    data::Value,
    expr::logical::{
        flexible::{FlexibleExpression, Iterable},
        stringy::StringyWithList,
    },
    utils::NapiBox,
};
use boolish::Boolish;
use comparable::Comparable;
use napi_derive::napi;
use stringy::Stringy;

/// @internal
/// @hideconstructor
#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct LogicalExpression {
    expr: LogicalExpressionUnion,
}

impl LogicalExpression {
    pub(crate) fn null() -> Self {
        Self {
            expr: LogicalExpressionUnion::Null,
        }
    }

    pub(crate) fn field(name: String) -> Self {
        Self {
            expr: LogicalExpressionUnion::Field { name },
        }
    }

    pub(crate) fn literal(value: impl Into<Value>) -> Self {
        Self {
            expr: LogicalExpressionUnion::Literal {
                value: value.into(),
            },
        }
    }

    pub(crate) fn unary(op: UnaryOperator, expr: LogicalExpression) -> Self {
        Self {
            expr: LogicalExpressionUnion::Unary {
                op,
                expr: NapiBox(Box::new(expr)),
            },
        }
    }

    pub(crate) fn nary(op: NaryOp, exprs: Vec<LogicalExpression>) -> Self {
        Self {
            expr: LogicalExpressionUnion::Nary {
                op,
                exprs: exprs.into_iter().map(|e| NapiBox(Box::new(e))).collect(),
            },
        }
    }

    pub(crate) fn binary(
        op: BinaryOperator,
        left: LogicalExpression,
        right: LogicalExpression,
    ) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(left)),
                op,
                right: NapiBox(Box::new(right)),
            },
        }
    }

    pub(crate) fn ternary(
        op: TernaryOperator,
        x: LogicalExpression,
        y: LogicalExpression,
        z: LogicalExpression,
    ) -> Self {
        Self {
            expr: LogicalExpressionUnion::Ternary {
                op,
                x: NapiBox(Box::new(x)),
                y: NapiBox(Box::new(y)),
                z: NapiBox(Box::new(z)),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum LogicalExpressionUnion {
    Null,
    Field {
        name: String,
    },
    Literal {
        value: Value,
    },
    Unary {
        op: UnaryOperator,
        expr: NapiBox<LogicalExpression>,
    },
    Binary {
        left: NapiBox<LogicalExpression>,
        op: BinaryOperator,
        right: NapiBox<LogicalExpression>,
    },
    Ternary {
        op: TernaryOperator,
        x: NapiBox<LogicalExpression>,
        y: NapiBox<LogicalExpression>,
        z: NapiBox<LogicalExpression>,
    },
    Nary {
        op: NaryOp,
        exprs: Vec<NapiBox<LogicalExpression>>,
    },
}

impl From<bool> for LogicalExpressionUnion {
    fn from(value: bool) -> Self {
        LogicalExpressionUnion::Literal {
            value: Value::Bool(value),
        }
    }
}

impl From<i64> for LogicalExpressionUnion {
    fn from(value: i64) -> Self {
        LogicalExpressionUnion::Literal {
            value: Value::I64(value),
        }
    }
}

impl From<f64> for LogicalExpressionUnion {
    fn from(value: f64) -> Self {
        LogicalExpressionUnion::Literal {
            value: Value::F64(value),
        }
    }
}

#[napi(namespace = "query")]
impl LogicalExpression {
    /// Returns a string representation of the logical expression.
    #[napi]
    pub fn to_string(&self) -> String {
        format!("LogicalExpression({:?})", self.expr)
    }

    // Unary operators

    /// Checks if the expression evaluates to null.
    #[napi]
    pub fn is_null(&self) -> Self {
        Self::unary(UnaryOperator::IsNull, self.clone())
    }

    /// Checks if the expression evaluates to a non-null value.
    #[napi]
    pub fn is_not_null(&self) -> Self {
        Self::unary(UnaryOperator::IsNotNull, self.clone())
    }

    /// Computes the absolute value of the expression.
    #[napi]
    pub fn abs(&self) -> Self {
        Self::unary(UnaryOperator::Abs, self.clone())
    }

    /// Computes the natural logarithm of the expression.
    #[napi]
    pub fn ln(&self) -> Self {
        Self::unary(UnaryOperator::Ln, self.clone())
    }

    /// Computes the exponential of the expression.
    #[napi]
    pub fn exp(&self) -> Self {
        Self::unary(UnaryOperator::Exp, self.clone())
    }

    /// Computes the square root of the expression.
    #[napi]
    pub fn sqrt(&self) -> Self {
        Self::unary(UnaryOperator::Sqrt, self.clone())
    }

    /// Computes the square of the expression.
    #[napi]
    pub fn square(&self) -> Self {
        Self::unary(UnaryOperator::Square, self.clone())
    }

    // Comparison operators

    /// Checks if the expression equals another value.
    #[napi]
    pub fn eq(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        other: Comparable,
    ) -> Self {
        Self::binary(BinaryOperator::Eq, self.clone(), other.into())
    }

    /// Checks if the expression does not equal another value.
    #[napi]
    pub fn ne(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        other: Comparable,
    ) -> Self {
        Self::binary(BinaryOperator::Neq, self.clone(), other.into())
    }

    /// Checks if the expression is less than another value.
    #[napi]
    pub fn lt(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | number | string")] other: Ordered,
    ) -> Self {
        Self::binary(BinaryOperator::Lt, self.clone(), other.into())
    }

    /// Checks if the expression is less than or equal to another value.
    #[napi]
    pub fn lte(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | number | string")] other: Ordered,
    ) -> Self {
        Self::binary(BinaryOperator::Lte, self.clone(), other.into())
    }

    /// Checks if the expression is greater than another value.
    #[napi]
    pub fn gt(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | number | string")] other: Ordered,
    ) -> Self {
        Self::binary(BinaryOperator::Gt, self.clone(), other.into())
    }

    /// Checks if the expression is greater than or equal to another value.
    #[napi]
    pub fn gte(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | number | string")] other: Ordered,
    ) -> Self {
        Self::binary(BinaryOperator::Gte, self.clone(), other.into())
    }

    /// Adds another value to the expression.
    #[napi]
    pub fn add(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Add, self.clone(), other.into())
    }

    /// Subtracts another value from the expression.
    #[napi]
    pub fn sub(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Sub, self.clone(), other.into())
    }

    /// Multiplies the expression by another value.
    #[napi]
    pub fn mul(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Mul, self.clone(), other.into())
    }

    /// Divides the expression by another value.
    #[napi]
    pub fn div(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Div, self.clone(), other.into())
    }

    /// Computes the minimum of the expression and another value.
    #[napi]
    pub fn min(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | number | string")] other: Ordered,
    ) -> Self {
        Self::binary(BinaryOperator::Min, self.clone(), other.into())
    }

    /// Computes the maximum of the expression and another value.
    #[napi]
    pub fn max(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | number | string")] other: Ordered,
    ) -> Self {
        Self::binary(BinaryOperator::Max, self.clone(), other.into())
    }

    /// Computes the logical AND of the expression and another expression.
    #[napi]
    pub fn and(&self, #[napi(ts_arg_type = "LogicalExpression | boolean")] other: Boolish) -> Self {
        Self::binary(BinaryOperator::And, self.clone(), other.into())
    }

    /// Computes the logical OR of the expression and another expression.
    #[napi]
    pub fn or(&self, #[napi(ts_arg_type = "LogicalExpression | boolean")] other: Boolish) -> Self {
        Self::binary(BinaryOperator::Or, self.clone(), other.into())
    }

    /// Checks if the expression starts with another value.
    #[napi]
    pub fn starts_with(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string")] other: Stringy,
    ) -> Self {
        Self::binary(BinaryOperator::StartsWith, self.clone(), other.into())
    }

    /// Checks if the expression contains another value.
    #[napi]
    pub fn contains(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | number")] other: FlexibleExpression,
    ) -> Self {
        Self::binary(BinaryOperator::Contains, self.clone(), other.into())
    }

    /// Checks if the expression is in another value.
    #[napi(js_name = "in")]
    pub fn in_(
        &self,
        #[napi(
            ts_arg_type = "LogicalExpression | string | Array<string> | Array<number> | data.List"
        )]
        other: Iterable,
    ) -> Self {
        Self::binary(BinaryOperator::In, self.clone(), other.into())
    }

    /// Checks if the expression matches all terms against the field with keyword index.
    #[napi]
    pub fn match_all(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | string[]")] other: StringyWithList,
    ) -> Self {
        Self::binary(BinaryOperator::MatchAll, self.clone(), other.into())
    }

    /// Checks if the expression matches any term against the field with keyword index.
    #[napi]
    pub fn match_any(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | string[]")] other: StringyWithList,
    ) -> Self {
        Self::binary(BinaryOperator::MatchAny, self.clone(), other.into())
    }

    /// Coalesces nulls in the expression with another value.
    #[napi]
    pub fn coalesce(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric,
    ) -> Self {
        Self::binary(BinaryOperator::Coalesce, self.clone(), other.into())
    }

    /// Chooses between two values based on the expression.
    #[napi]
    pub fn choose(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        x: Comparable,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        y: Comparable,
    ) -> Self {
        Self::ternary(TernaryOperator::Choose, self.clone(), x.into(), y.into())
    }

    /// Multiplies the scoring expression by the provided `boost` value if the `condition` is true.
    /// Otherwise, the scoring expression is unchanged (multiplied by 1).
    #[napi]
    pub fn boost(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | boolean")] condition: Boolish,
        #[napi(ts_arg_type = "LogicalExpression | number")] boost: Numeric,
    ) -> Self {
        let condition_expr: LogicalExpression = condition.into();
        let boost_expr: LogicalExpression = boost.into();
        let one_expr: LogicalExpression = LogicalExpression::literal(1);
        let choose_expr: LogicalExpression = condition_expr.choose(
            comparable::Comparable::Expr(boost_expr),
            comparable::Comparable::Expr(one_expr),
        );
        Self::binary(BinaryOperator::Mul, self.clone(), choose_expr)
    }

    /// Check if the expression matches the provided regexp pattern.
    #[napi]
    pub fn regexp_match(
        &self,
        #[napi(ts_arg_type = "string")] other: String,
        #[napi(ts_arg_type = "string | null")] flags: Option<String>,
    ) -> Self {
        Self::ternary(
            TernaryOperator::RegexpMatch,
            self.clone(),
            LogicalExpression::literal(other),
            LogicalExpression::literal(flags.map(Value::String).unwrap_or(Value::Null)),
        )
    }
}

impl Into<topk_rs::proto::v1::data::LogicalExpr> for LogicalExpression {
    fn into(self) -> topk_rs::proto::v1::data::LogicalExpr {
        match self.expr {
            LogicalExpressionUnion::Null => topk_rs::proto::v1::data::LogicalExpr::literal(
                topk_rs::proto::v1::data::Value::null(),
            ),
            LogicalExpressionUnion::Field { name } => {
                topk_rs::proto::v1::data::LogicalExpr::field(name)
            }
            LogicalExpressionUnion::Literal { value } => {
                topk_rs::proto::v1::data::LogicalExpr::literal(value)
            }
            LogicalExpressionUnion::Unary { op, expr } => {
                topk_rs::proto::v1::data::LogicalExpr::unary(op, expr.as_ref().clone())
            }
            LogicalExpressionUnion::Binary { left, op, right } => {
                topk_rs::proto::v1::data::LogicalExpr::binary(
                    op,
                    left.as_ref().clone(),
                    right.as_ref().clone(),
                )
            }
            LogicalExpressionUnion::Ternary { op, x, y, z } => {
                topk_rs::proto::v1::data::LogicalExpr::ternary(
                    op,
                    x.as_ref().clone(),
                    y.as_ref().clone(),
                    z.as_ref().clone(),
                )
            }
            LogicalExpressionUnion::Nary { op, exprs } => {
                topk_rs::proto::v1::data::LogicalExpr::nary(
                    op,
                    exprs.into_iter().map(|e| e.as_ref().clone()),
                )
            }
        }
    }
}
