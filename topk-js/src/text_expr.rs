use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_protos::v1::data;

use crate::my_box::MyBox;

#[napi]
#[derive(Debug, Clone)]
pub enum TextExpression {
  Terms {
    all: bool,
    terms: Vec<Term>,
  },
  And {
    #[napi(ts_type = "TextExpression")]
    left: MyBox<TextExpression>,
    #[napi(ts_type = "TextExpression")]
    right: MyBox<TextExpression>,
  },
  Or {
    #[napi(ts_type = "TextExpression")]
    left: MyBox<TextExpression>,
    #[napi(ts_type = "TextExpression")]
    right: MyBox<TextExpression>,
  },
}

impl Into<data::TextExpr> for TextExpression {
  fn into(self) -> data::TextExpr {
    match self {
      TextExpression::Terms { all, terms } => {
        data::TextExpr::terms(all, terms.into_iter().map(|t| t.into()).collect())
      }
      TextExpression::And { left, right } => {
        let left_expr: data::TextExpr = left.as_ref().clone().into();
        let right_expr: data::TextExpr = right.as_ref().clone().into();
        data::TextExpr::and(left_expr, right_expr)
      }
      TextExpression::Or { left, right } => {
        let left_expr: data::TextExpr = left.as_ref().clone().into();
        let right_expr: data::TextExpr = right.as_ref().clone().into();
        data::TextExpr::or(left_expr, right_expr)
      }
    }
  }
}

#[napi]
#[derive(Debug, Clone)]
pub struct Term {
  pub token: String,
  pub field: Option<String>,
  pub weight: f64,
}

impl Into<data::text_expr::Term> for Term {
  fn into(self) -> data::text_expr::Term {
    data::text_expr::Term {
      token: self.token,
      field: self.field,
      weight: self.weight as f32,
    }
  }
}

impl FromNapiValue for Term {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
  ) -> Result<Self, napi::Status> {
    let object = Object::from_napi_value(env, value)?;
    Ok(Self {
      token: object.get("token".into())?.unwrap_or_default(),
      field: object.get("field".into())?,
      weight: object.get("weight".into())?.unwrap_or(1.0),
    })
  }
}
