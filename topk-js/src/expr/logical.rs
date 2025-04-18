use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::data::{napi_box::NapiBox, scalar::Scalar};

use super::flexible::{Boolish, FlexibleExpr, Numeric, Stringy};

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub enum LogicalExpressionUnion {
    Null,
    Field {
        name: String,
    },
    Literal {
        #[napi(ts_type = "number | string | boolean")]
        value: Scalar,
    },
    Unary {
        op: UnaryOperator,
        #[napi(ts_type = "LogicalExpression")]
        expr: NapiBox<LogicalExpressionUnion>,
    },
    Binary {
        #[napi(ts_type = "LogicalExpression")]
        left: NapiBox<LogicalExpressionUnion>,
        op: BinaryOperator,
        #[napi(ts_type = "LogicalExpression")]
        right: NapiBox<LogicalExpressionUnion>,
    },
}

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct LogicalExpression {
    expr: LogicalExpressionUnion,
}

#[napi(namespace = "query")]
impl LogicalExpression {
    #[napi(factory)]
    pub fn create(expr: LogicalExpressionUnion) -> Self {
        LogicalExpression { expr }
    }

    #[napi(getter)]
    pub fn get_expr(&self) -> LogicalExpressionUnion {
        self.expr.clone()
    }

    #[napi]
    pub fn eq(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        other: FlexibleExpr,
    ) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Eq,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn ne(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string | number | boolean | null | undefined")]
        other: FlexibleExpr,
    ) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Neq,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn lt(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Lt,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn lte(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Lte,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn gt(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Gt,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn gte(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Gte,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn add(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Add,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn sub(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Sub,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn mul(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Mul,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn div(&self, #[napi(ts_arg_type = "LogicalExpression | number")] other: Numeric) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Div,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn and(&self, #[napi(ts_arg_type = "LogicalExpression | boolean")] other: Boolish) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::And,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn or(&self, #[napi(ts_arg_type = "LogicalExpression | boolean")] other: Boolish) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Or,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn starts_with(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string")] other: Stringy,
    ) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::StartsWith,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }

    #[napi]
    pub fn contains(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | string")] other: Stringy,
    ) -> Self {
        Self {
            expr: LogicalExpressionUnion::Binary {
                left: NapiBox(Box::new(self.expr.clone())),
                op: BinaryOperator::Contains,
                right: NapiBox(Box::new(other.into())),
            },
        }
    }
}

impl Into<topk_protos::v1::data::LogicalExpr> for LogicalExpression {
    fn into(self) -> topk_protos::v1::data::LogicalExpr {
        self.expr.into()
    }
}

impl Into<topk_rs::expr::logical::LogicalExpr> for LogicalExpression {
    fn into(self) -> topk_rs::expr::logical::LogicalExpr {
        self.expr.into()
    }
}

impl Into<topk_protos::v1::data::LogicalExpr> for LogicalExpressionUnion {
    fn into(self) -> topk_protos::v1::data::LogicalExpr {
        match self {
            LogicalExpressionUnion::Null => {
                topk_protos::v1::data::LogicalExpr::literal(topk_protos::v1::data::Value::null())
            }
            LogicalExpressionUnion::Field { name } => {
                topk_protos::v1::data::LogicalExpr::field(name)
            }
            LogicalExpressionUnion::Literal { value } => {
                topk_protos::v1::data::LogicalExpr::literal(value.into())
            }
            LogicalExpressionUnion::Unary { op, expr } => {
                topk_protos::v1::data::LogicalExpr::unary(op.into(), expr.as_ref().clone().into())
            }
            LogicalExpressionUnion::Binary { left, op, right } => {
                topk_protos::v1::data::LogicalExpr::binary(
                    op.into(),
                    left.as_ref().clone().into(),
                    right.as_ref().clone().into(),
                )
            }
        }
    }
}

impl Into<topk_rs::expr::logical::LogicalExpr> for LogicalExpressionUnion {
    fn into(self) -> topk_rs::expr::logical::LogicalExpr {
        match self {
            LogicalExpressionUnion::Null => topk_rs::expr::logical::LogicalExpr::Null {},
            LogicalExpressionUnion::Field { name } => {
                topk_rs::expr::logical::LogicalExpr::Field { name }
            }
            LogicalExpressionUnion::Literal { value } => {
                topk_rs::expr::logical::LogicalExpr::Literal {
                    value: value.into(),
                }
            }
            LogicalExpressionUnion::Unary { op, expr } => {
                topk_rs::expr::logical::LogicalExpr::Unary {
                    op: op.into(),
                    expr: Box::new(expr.as_ref().clone().into()),
                }
            }
            LogicalExpressionUnion::Binary { left, op, right } => {
                topk_rs::expr::logical::LogicalExpr::Binary {
                    left: Box::new(left.as_ref().clone().into()),
                    op: op.into(),
                    right: Box::new(right.as_ref().clone().into()),
                }
            }
        }
    }
}

impl FromNapiValue for LogicalExpression {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        let logical_expression = LogicalExpression::from_napi_ref(env, value)?;
        let expr = logical_expression.expr.clone();

        Ok(Self { expr })
    }
}

#[napi(string_enum = "camelCase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    // Logical ops
    And,
    Or,
    // Comparison ops
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    StartsWith,
    Contains,
    // Arithmetic ops
    Add,
    Sub,
    Mul,
    Div,
}

impl Into<topk_protos::v1::data::logical_expr::binary_op::Op> for BinaryOperator {
    fn into(self) -> topk_protos::v1::data::logical_expr::binary_op::Op {
        match self {
            BinaryOperator::And => topk_protos::v1::data::logical_expr::binary_op::Op::And,
            BinaryOperator::Or => topk_protos::v1::data::logical_expr::binary_op::Op::Or,
            BinaryOperator::Eq => topk_protos::v1::data::logical_expr::binary_op::Op::Eq,
            BinaryOperator::Neq => topk_protos::v1::data::logical_expr::binary_op::Op::Neq,
            BinaryOperator::Lt => topk_protos::v1::data::logical_expr::binary_op::Op::Lt,
            BinaryOperator::Lte => topk_protos::v1::data::logical_expr::binary_op::Op::Lte,
            BinaryOperator::Gt => topk_protos::v1::data::logical_expr::binary_op::Op::Gt,
            BinaryOperator::Gte => topk_protos::v1::data::logical_expr::binary_op::Op::Gte,
            BinaryOperator::StartsWith => {
                topk_protos::v1::data::logical_expr::binary_op::Op::StartsWith
            }
            BinaryOperator::Contains => {
                topk_protos::v1::data::logical_expr::binary_op::Op::Contains
            }
            BinaryOperator::Add => topk_protos::v1::data::logical_expr::binary_op::Op::Add,
            BinaryOperator::Sub => topk_protos::v1::data::logical_expr::binary_op::Op::Sub,
            BinaryOperator::Mul => topk_protos::v1::data::logical_expr::binary_op::Op::Mul,
            BinaryOperator::Div => topk_protos::v1::data::logical_expr::binary_op::Op::Div,
        }
    }
}

impl Into<topk_rs::expr::logical::BinaryOperator> for BinaryOperator {
    fn into(self) -> topk_rs::expr::logical::BinaryOperator {
        match self {
            BinaryOperator::And => topk_rs::expr::logical::BinaryOperator::And,
            BinaryOperator::Or => topk_rs::expr::logical::BinaryOperator::Or,
            BinaryOperator::Eq => topk_rs::expr::logical::BinaryOperator::Eq,
            BinaryOperator::Neq => topk_rs::expr::logical::BinaryOperator::NotEq,
            BinaryOperator::Lt => topk_rs::expr::logical::BinaryOperator::Lt,
            BinaryOperator::Lte => topk_rs::expr::logical::BinaryOperator::LtEq,
            BinaryOperator::Gt => topk_rs::expr::logical::BinaryOperator::Gt,
            BinaryOperator::Gte => topk_rs::expr::logical::BinaryOperator::GtEq,
            BinaryOperator::StartsWith => topk_rs::expr::logical::BinaryOperator::StartsWith,
            BinaryOperator::Contains => topk_rs::expr::logical::BinaryOperator::Contains,
            BinaryOperator::Add => topk_rs::expr::logical::BinaryOperator::Add,
            BinaryOperator::Sub => topk_rs::expr::logical::BinaryOperator::Sub,
            BinaryOperator::Mul => topk_rs::expr::logical::BinaryOperator::Mul,
            BinaryOperator::Div => topk_rs::expr::logical::BinaryOperator::Div,
        }
    }
}

#[napi(string_enum = "camelCase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not,
    IsNull,
    IsNotNull,
}

impl Into<topk_protos::v1::data::logical_expr::unary_op::Op> for UnaryOperator {
    fn into(self) -> topk_protos::v1::data::logical_expr::unary_op::Op {
        match self {
            UnaryOperator::Not => topk_protos::v1::data::logical_expr::unary_op::Op::Not,
            UnaryOperator::IsNull => topk_protos::v1::data::logical_expr::unary_op::Op::IsNull,
            UnaryOperator::IsNotNull => {
                topk_protos::v1::data::logical_expr::unary_op::Op::IsNotNull
            }
        }
    }
}

impl Into<topk_rs::expr::logical::UnaryOperator> for UnaryOperator {
    fn into(self) -> topk_rs::expr::logical::UnaryOperator {
        match self {
            UnaryOperator::Not => topk_rs::expr::logical::UnaryOperator::Not,
            UnaryOperator::IsNull => topk_rs::expr::logical::UnaryOperator::IsNull,
            UnaryOperator::IsNotNull => topk_rs::expr::logical::UnaryOperator::IsNotNull,
        }
    }
}
