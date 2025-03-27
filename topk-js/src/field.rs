use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{
  binary_expr::BinaryOperator, document::Value, logical_expr::LogicalExpression, my_box::MyBox,
};

#[napi]
pub struct Field {
  expr: LogicalExpression,
}

#[napi]
impl Field {
  #[napi(factory)]
  pub fn create(name: String) -> Self {
    Self {
      expr: LogicalExpression::Field { name },
    }
  }

  #[napi]
  pub fn gt(&self, value: f64) -> Field {
    Field {
      expr: LogicalExpression::Binary {
        left: MyBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Gt,
        right: MyBox(Box::new(LogicalExpression::Literal {
          value: Value::F64(value as f64),
        })),
      },
    }
  }

  #[napi]
  pub fn eq(&self, value: Value) -> Field {
    Field {
      expr: LogicalExpression::Binary {
        left: MyBox(Box::new(self.expr.clone())),
        op: BinaryOperator::Eq,
        right: MyBox(Box::new(LogicalExpression::Literal { value })),
      },
    }
  }

  #[napi(getter)]
  pub fn get_expr(&self) -> LogicalExpression {
    self.expr.clone()
  }
}

#[napi]
pub fn field(name: String) -> Field {
  Field::create(name)
}

impl FromNapiValue for Field {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
  ) -> Result<Self, napi::Status> {
    let object = Object::from_napi_value(env, value)?;
    let expr: LogicalExpression = object.get("expr")?.ok_or_else(|| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "Field object missing 'expr' property".to_string(),
      )
    })?;

    Ok(Self { expr })
  }
}
