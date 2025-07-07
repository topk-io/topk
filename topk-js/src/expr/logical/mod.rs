mod binary_op;
mod boolish;
mod comparable;
mod numeric;
mod stringy;
mod ternary_op;
mod unary_op;

pub use binary_op::BinaryOperator;
pub use ternary_op::TernaryOperator;
pub use unary_op::UnaryOperator;

use crate::{data::Scalar, utils::NapiBox};
use boolish::Boolish;
use comparable::Comparable;
use napi_derive::napi;
use numeric::Numeric;
use stringy::Stringy;

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

    pub(crate) fn literal(value: impl Into<Scalar>) -> Self {
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
        value: Scalar,
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
}

impl From<bool> for LogicalExpressionUnion {
    fn from(value: bool) -> Self {
        LogicalExpressionUnion::Literal {
            value: Scalar::Bool(value),
        }
    }
}

impl From<i64> for LogicalExpressionUnion {
    fn from(value: i64) -> Self {
        LogicalExpressionUnion::Literal {
            value: Scalar::I64(value),
        }
    }
}

impl From<f64> for LogicalExpressionUnion {
    fn from(value: f64) -> Self {
        LogicalExpressionUnion::Literal {
            value: Scalar::F64(value),
        }
    }
}

#[napi(namespace = "query")]
impl LogicalExpression {
    #[napi]
    pub fn to_string(&self) -> String {
        format!("LogicalExpression({:?})", self.expr)
    }

    // Unary operators

    #[napi]
    pub fn is_null(&self) -> Self {
        Self::unary(UnaryOperator::IsNull, self.clone())
    }

    #[napi]
    pub fn is_not_null(&self) -> Self {
        Self::unary(UnaryOperator::IsNotNull, self.clone())
    }

    // Comparison operators

    #[napi]
    pub fn eq(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        other: Comparable,
    ) -> Self {
        Self::binary(BinaryOperator::Eq, self.clone(), other.into())
    }

    #[napi]
    pub fn ne(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        other: Comparable,
    ) -> Self {
        Self::binary(BinaryOperator::Neq, self.clone(), other.into())
    }

    #[napi]
    pub fn lt(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Lt, self.clone(), other.into())
    }

    #[napi]
    pub fn lte(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Lte, self.clone(), other.into())
    }

    #[napi]
    pub fn gt(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Gt, self.clone(), other.into())
    }

    #[napi]
    pub fn gte(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Gte, self.clone(), other.into())
    }

    #[napi]
    pub fn add(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Add, self.clone(), other.into())
    }

    #[napi]
    pub fn sub(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Sub, self.clone(), other.into())
    }

    #[napi]
    pub fn mul(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Mul, self.clone(), other.into())
    }

    #[napi]
    pub fn div(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self::binary(BinaryOperator::Div, self.clone(), other.into())
    }

    #[napi]
    pub fn and(&self, #[napi(ts_arg_type = "LogicalExpression | boolean")] other: Boolish) -> Self {
        Self::binary(BinaryOperator::And, self.clone(), other.into())
    }

    #[napi]
    pub fn or(&self, #[napi(ts_arg_type = "LogicalExpression | boolean")] other: Boolish) -> Self {
        Self::binary(BinaryOperator::Or, self.clone(), other.into())
    }

    #[napi]
    pub fn starts_with(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string")] other: Stringy,
    ) -> Self {
        Self::binary(BinaryOperator::StartsWith, self.clone(), other.into())
    }

    #[napi]
    pub fn contains(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string")] other: Stringy,
    ) -> Self {
        Self::binary(BinaryOperator::Contains, self.clone(), other.into())
    }

    #[napi]
    pub fn match_all(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string")] other: Stringy,
    ) -> Self {
        Self::binary(BinaryOperator::MatchAll, self.clone(), other.into())
    }

    #[napi]
    pub fn match_any(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string")] other: Stringy,
    ) -> Self {
        Self::binary(BinaryOperator::MatchAny, self.clone(), other.into())
    }

    #[napi]
    pub fn coalesce(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric,
    ) -> Self {
        Self::binary(BinaryOperator::Coalesce, self.clone(), other.into())
    }

    #[napi]
    pub fn choose(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        y: Comparable,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        z: Comparable,
    ) -> Self {
        Self::ternary(TernaryOperator::Choose, self.clone(), y.into(), z.into())
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
            comparable::Comparable::Expr(one_expr)
        );
        Self::binary(BinaryOperator::Mul, self.clone(), choose_expr)
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
        }
    }
}
