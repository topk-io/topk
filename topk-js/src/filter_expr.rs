use super::{logical_expr::LogicalExpression, text_expr::TextExpression};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_protos::v1::data;

#[derive(Debug, Clone)]
pub enum FilterExpressionUnion {
  Logical { expr: LogicalExpression },
  Text { expr: TextExpression },
}

impl Into<data::stage::filter_stage::FilterExpr> for FilterExpressionUnion {
  fn into(self) -> data::stage::filter_stage::FilterExpr {
    match self {
      FilterExpressionUnion::Logical { expr } => {
        data::stage::filter_stage::FilterExpr::logical(expr.into())
      }
      FilterExpressionUnion::Text { expr } => {
        data::stage::filter_stage::FilterExpr::text(expr.into())
      }
    }
  }
}

impl FromNapiValue for FilterExpressionUnion {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
  ) -> Result<Self, napi::Status> {
    let env_env = Env::from_raw(env);

    let is_logical_expression = {
      let env_value = Unknown::from_napi_value(env, value)?;
      LogicalExpression::instance_of(env_env, env_value)?
    };

    let is_text_expression = {
      let env_value = Unknown::from_napi_value(env, value)?;
      TextExpression::instance_of(env_env, env_value)?
    };

    if is_logical_expression {
      Ok(FilterExpressionUnion::Logical {
        expr: LogicalExpression::from_napi_value(env, value)?,
      })
    } else if is_text_expression {
      Ok(FilterExpressionUnion::Text {
        expr: TextExpression::from_napi_value(env, value)?,
      })
    } else {
      unreachable!("Value must be either a LogicalExpression or TextExpression")
    }
  }
}

impl ToNapiValue for FilterExpressionUnion {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> napi::Result<napi::sys::napi_value> {
    match val {
      FilterExpressionUnion::Logical { expr } => LogicalExpression::to_napi_value(env, expr),
      FilterExpressionUnion::Text { expr } => TextExpression::to_napi_value(env, expr),
    }
  }
}
