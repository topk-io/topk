use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{
  binary_expr::BinaryOperator, document::Value, napi_box::NapiBox, unary_expr::UnaryOperator,
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

  #[napi]
  pub fn eq(&self, value: Value) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Eq,
        right: NapiBox(Box::new(LogicalExpressionUnion::Literal { value })),
      },
    }
  }

  // TODO: Remove this
  #[napi(getter)]
  pub fn get_expr(&self) -> LogicalExpressionUnion {
    self.expr.clone()
  }

  #[napi]
  pub fn neq(&self, value: Value) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Neq,
        right: NapiBox(Box::new(LogicalExpressionUnion::Literal { value })),
      },
    }
  }

  #[napi]
  pub fn lt(&self, value: Value) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Lt,
        right: NapiBox(Box::new(LogicalExpressionUnion::Literal { value })),
      },
    }
  }

  #[napi]
  pub fn lte(&self, value: Value) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Lte,
        right: NapiBox(Box::new(LogicalExpressionUnion::Literal { value })),
      },
    }
  }

  #[napi]
  pub fn gt(&self, value: Value) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Gt,
        right: NapiBox(Box::new(LogicalExpressionUnion::Literal { value })),
      },
    }
  }

  #[napi]
  pub fn gte(&self, value: Value) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Gte,
        right: NapiBox(Box::new(LogicalExpressionUnion::Literal { value })),
      },
    }
  }

  #[napi]
  pub fn add(&self, other: &LogicalExpression) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Add,
        right: NapiBox(Box::new(other.expr.clone())),
      },
    }
  }

  #[napi]
  pub fn sub(&self, other: &LogicalExpression) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Sub,
        right: NapiBox(Box::new(other.expr.clone())),
      },
    }
  }

  #[napi]
  pub fn mul(&self, other: &LogicalExpression) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Mul,
        right: NapiBox(Box::new(other.expr.clone())),
      },
    }
  }

  #[napi]
  pub fn div(&self, other: &LogicalExpression) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Div,
        right: NapiBox(Box::new(other.expr.clone())),
      },
    }
  }

  #[napi]
  pub fn and(&self, other: &LogicalExpression) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::And,
        right: NapiBox(Box::new(other.expr.clone())),
      },
    }
  }

  #[napi]
  pub fn or(&self, other: &LogicalExpression) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Or,
        right: NapiBox(Box::new(other.expr.clone())),
      },
    }
  }

  #[napi]
  pub fn starts_with(&self, other: &LogicalExpression) -> Self {
    Self {
      expr: LogicalExpressionUnion::Binary {
        left: NapiBox(Box::new(self.expr.clone())),
        op: BinaryOperator::StartsWith,
        right: NapiBox(Box::new(other.expr.clone())),
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
      LogicalExpressionUnion::Null => unreachable!(),
      LogicalExpressionUnion::Field { name } => topk_protos::v1::data::LogicalExpr::field(name),
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
    let object = Object::from_napi_value(env, value)?;
    let expr: LogicalExpressionUnion = object.get("expr")?.ok_or_else(|| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "LogicalExpression object missing 'expr' property".to_string(),
      )
    })?;

    Ok(Self { expr })
  }
}
