use crate::utils::NapiBox;
use napi_derive::napi;
use topk_rs::proto::v1::data;

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct TextExpression {
    expr: TextExpressionUnion,
}

#[derive(Debug, Clone)]
pub enum TextExpressionUnion {
    Terms {
        all: bool,
        terms: Vec<Term>,
    },
    And {
        left: NapiBox<TextExpression>,
        right: NapiBox<TextExpression>,
    },
    Or {
        left: NapiBox<TextExpression>,
        right: NapiBox<TextExpression>,
    },
}

impl TextExpression {
    pub(crate) fn terms(all: bool, terms: Vec<Term>) -> Self {
        Self {
            expr: TextExpressionUnion::Terms { all, terms },
        }
    }
}

#[napi(namespace = "query")]
impl TextExpression {
    #[napi]
    pub fn and(&self, other: &TextExpression) -> Self {
        TextExpression {
            expr: TextExpressionUnion::And {
                left: NapiBox(Box::new(self.clone())),
                right: NapiBox(Box::new(other.clone())),
            },
        }
    }

    #[napi]
    pub fn or(&self, other: &TextExpression) -> Self {
        TextExpression {
            expr: TextExpressionUnion::Or {
                left: NapiBox(Box::new(self.clone())),
                right: NapiBox(Box::new(other.clone())),
            },
        }
    }
}

impl Into<topk_rs::proto::v1::data::TextExpr> for TextExpression {
    fn into(self) -> topk_rs::proto::v1::data::TextExpr {
        match self.expr {
            TextExpressionUnion::Terms { all, terms } => topk_rs::proto::v1::data::TextExpr::terms(
                all,
                terms.into_iter().map(|t| t.into()).collect(),
            ),
            TextExpressionUnion::And { left, right } => {
                let left_expr: topk_rs::proto::v1::data::TextExpr = left.as_ref().clone().into();
                let right_expr: topk_rs::proto::v1::data::TextExpr = right.as_ref().clone().into();
                topk_rs::proto::v1::data::TextExpr::and(left_expr, right_expr)
            }
            TextExpressionUnion::Or { left, right } => {
                let left_expr: topk_rs::proto::v1::data::TextExpr = left.as_ref().clone().into();
                let right_expr: topk_rs::proto::v1::data::TextExpr = right.as_ref().clone().into();
                topk_rs::proto::v1::data::TextExpr::or(left_expr, right_expr)
            }
        }
    }
}

impl Into<topk_rs::expr::text::TextExpr> for TextExpression {
    fn into(self) -> topk_rs::expr::text::TextExpr {
        match self.expr {
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
