use super::{logical_expr::LogicalExpression, text_expr::TextExpression};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_protos::v1::data;

#[napi]
#[derive(Debug, Clone)]
pub enum FilterExpressionUnion {
  Logical { expr: LogicalExpression },
  Text { expr: TextExpression },
}

#[napi]
#[derive(Debug, Clone)]
pub struct FilterExpression {
  expr: FilterExpressionUnion,
}

#[napi]
impl FilterExpression {
  #[napi(factory)]
  pub fn create(expr: FilterExpressionUnion) -> Self {
    Self { expr }
  }

  // TODO
  #[napi(getter)]
  pub fn get_expr(&self) -> FilterExpressionUnion {
    self.expr.clone()
  }
}

impl Into<data::stage::filter_stage::FilterExpr> for FilterExpression {
  fn into(self) -> data::stage::filter_stage::FilterExpr {
    match self.expr {
      FilterExpressionUnion::Logical { expr } => {
        data::stage::filter_stage::FilterExpr::logical(expr.into())
      }
      FilterExpressionUnion::Text { expr } => {
        data::stage::filter_stage::FilterExpr::text(expr.into())
      }
    }
  }
}

impl FromNapiValue for FilterExpression {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
  ) -> Result<Self, napi::Status> {
    let object = Object::from_napi_value(env, value)?;
    let expr: FilterExpressionUnion = object.get("expr")?.ok_or_else(|| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "FilterExpression object missing 'expr' property".to_string(),
      )
    })?;

    Ok(Self { expr })
  }
}
