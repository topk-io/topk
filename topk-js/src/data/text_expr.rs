use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_protos::v1::data;

use super::napi_box::NapiBox;

#[napi]
#[derive(Debug, Clone)]
pub enum TextExpressionUnion {
    Terms {
        all: bool,
        terms: Vec<Term>,
    },
    And {
        #[napi(ts_type = "TextExpression")]
        left: NapiBox<TextExpressionUnion>,
        #[napi(ts_type = "TextExpression")]
        right: NapiBox<TextExpressionUnion>,
    },
    Or {
        #[napi(ts_type = "TextExpression")]
        left: NapiBox<TextExpressionUnion>,
        #[napi(ts_type = "TextExpression")]
        right: NapiBox<TextExpressionUnion>,
    },
}

#[napi]
#[derive(Debug, Clone)]
pub struct TextExpression {
    expr: TextExpressionUnion,
}

#[napi]
impl TextExpression {
    #[napi(factory)]
    pub fn create(expr: TextExpressionUnion) -> Self {
        TextExpression { expr }
    }

    #[napi(getter)]
    pub fn get_expr(&self) -> TextExpressionUnion {
        self.expr.clone()
    }

    #[napi]
    pub fn and(&self, other: &TextExpression) -> Self {
        TextExpression {
            expr: TextExpressionUnion::And {
                left: NapiBox(Box::new(self.expr.clone())),
                right: NapiBox(Box::new(other.expr.clone())),
            },
        }
    }

    #[napi]
    pub fn or(&self, other: &TextExpression) -> Self {
        TextExpression {
            expr: TextExpressionUnion::Or {
                left: NapiBox(Box::new(self.expr.clone())),
                right: NapiBox(Box::new(other.expr.clone())),
            },
        }
    }
}

impl FromNapiValue for TextExpression {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let env_env = Env::from_raw(env);

        let is_text_expression = {
            let env_value = Unknown::from_napi_value(env, value)?;
            TextExpression::instance_of(env_env, env_value)?
        };

        if is_text_expression {
            let object = Object::from_napi_value(env, value)?;

            let expr: Option<TextExpressionUnion> = object.get("expr".into())?;

            match expr {
                Some(expr) => Ok(TextExpression { expr }),
                None => Err(napi::Error::new(
                    napi::Status::GenericFailure,
                    "expr is required",
                )),
            }
        } else {
            unreachable!("Value must be a TextExpression")
        }
    }
}

impl Into<data::TextExpr> for TextExpression {
    fn into(self) -> data::TextExpr {
        match self.expr {
            TextExpressionUnion::Terms { all, terms } => {
                data::TextExpr::terms(all, terms.into_iter().map(|t| t.into()).collect())
            }
            TextExpressionUnion::And { left, right } => {
                let left_expr: data::TextExpr = left.as_ref().clone().into();
                let right_expr: data::TextExpr = right.as_ref().clone().into();
                data::TextExpr::and(left_expr, right_expr)
            }
            TextExpressionUnion::Or { left, right } => {
                let left_expr: data::TextExpr = left.as_ref().clone().into();
                let right_expr: data::TextExpr = right.as_ref().clone().into();
                data::TextExpr::or(left_expr, right_expr)
            }
        }
    }
}

impl Into<data::TextExpr> for TextExpressionUnion {
    fn into(self) -> data::TextExpr {
        match self {
            TextExpressionUnion::Terms { all, terms } => {
                data::TextExpr::terms(all, terms.into_iter().map(|t| t.into()).collect())
            }
            TextExpressionUnion::And { left, right } => {
                let left_expr: data::TextExpr = left.as_ref().clone().into();
                let right_expr: data::TextExpr = right.as_ref().clone().into();
                data::TextExpr::and(left_expr, right_expr)
            }
            TextExpressionUnion::Or { left, right } => {
                let left_expr: data::TextExpr = left.as_ref().clone().into();
                let right_expr: data::TextExpr = right.as_ref().clone().into();
                data::TextExpr::or(left_expr, right_expr)
            }
        }
    }
}

#[napi(object)]
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
