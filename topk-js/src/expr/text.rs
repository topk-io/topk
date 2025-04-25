use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_protos::v1::data;

use crate::data::napi_box::NapiBox;

#[napi(namespace = "query")]
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

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct TextExpression {
    expr: TextExpressionUnion,
}

#[napi(namespace = "query")]
impl TextExpression {
    #[napi(factory)]
    pub fn create(expr: TextExpressionUnion) -> Self {
        TextExpression { expr }
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
            let text_expression = TextExpression::from_napi_ref(env, value)?;
            let expr = text_expression.expr.clone();

            Ok(TextExpression { expr })
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

impl Into<topk_rs::expr::text::TextExpr> for TextExpression {
    fn into(self) -> topk_rs::expr::text::TextExpr {
        self.expr.into()
    }
}

impl Into<topk_rs::expr::text::TextExpr> for TextExpressionUnion {
    fn into(self) -> topk_rs::expr::text::TextExpr {
        match self {
            TextExpressionUnion::Terms { all, terms } => topk_rs::expr::text::TextExpr::Terms {
                all,
                terms: terms
                    .into_iter()
                    .map(|t| topk_rs::expr::text::Term {
                        token: t.token,
                        field: t.field,
                        weight: t.weight as f32,
                    })
                    .collect(),
            },
            TextExpressionUnion::And { left, right } => topk_rs::expr::text::TextExpr::And {
                left: Box::new(left.as_ref().clone().into()),
                right: Box::new(right.as_ref().clone().into()),
            },
            TextExpressionUnion::Or { left, right } => topk_rs::expr::text::TextExpr::Or {
                left: Box::new(left.as_ref().clone().into()),
                right: Box::new(right.as_ref().clone().into()),
            },
        }
    }
}

#[napi(object, namespace = "query")]
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
