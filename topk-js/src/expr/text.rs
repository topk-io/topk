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
    /// Computes the logical AND of the expression and another text expression.
    pub fn and(&self, other: &TextExpression) -> Self {
        TextExpression {
            expr: TextExpressionUnion::And {
                left: NapiBox(Box::new(self.clone())),
                right: NapiBox(Box::new(other.clone())),
            },
        }
    }

    #[napi]
    /// Computes the logical OR of the expression and another text expression.
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
                terms.into_iter().map(|term| term.into()).collect(),
            ),
            TextExpressionUnion::And { left, right } => {
                let left: topk_rs::proto::v1::data::TextExpr = left.as_ref().clone().into();
                let right: topk_rs::proto::v1::data::TextExpr = right.as_ref().clone().into();
                left.and(right)
            }
            TextExpressionUnion::Or { left, right } => {
                let left: topk_rs::proto::v1::data::TextExpr = left.as_ref().clone().into();
                let right: topk_rs::proto::v1::data::TextExpr = right.as_ref().clone().into();
                left.or(right)
            }
        }
    }
}

#[napi(object, namespace = "query")]
#[derive(Debug, Clone)]
pub struct Term {
    /// The token to match.
    pub token: String,
    /// The field to match against.
    pub field: Option<String>,
    /// The weight of the term.
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
