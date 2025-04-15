use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::{
    binary_expr::BinaryOperator,
    flexible_expr::{Boolish, FlexibleExpr, Numeric, Stringy},
    napi_box::NapiBox,
    unary_expr::UnaryOperator,
    value::Value,
};

#[napi]
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

#[napi]
#[derive(Debug, Clone)]
pub struct LogicalExpression {
    expr: LogicalExpressionUnion,
}

#[napi]
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
}

impl Into<topk_protos::v1::data::LogicalExpr> for LogicalExpression {
    fn into(self) -> topk_protos::v1::data::LogicalExpr {
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
