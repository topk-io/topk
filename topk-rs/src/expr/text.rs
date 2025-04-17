#[derive(Debug, Clone)]
pub enum TextExpr {
    Terms {
        all: bool,
        terms: Vec<Term>,
    },
    And {
        left: Box<TextExpr>,
        right: Box<TextExpr>,
    },
    Or {
        left: Box<TextExpr>,
        right: Box<TextExpr>,
    },
}

impl TextExpr {
    pub fn and(&self, other: TextExpr) -> Self {
        Self::And {
            left: Box::new(self.clone()),
            right: Box::new(other),
        }
    }

    pub fn or(&self, other: TextExpr) -> Self {
        Self::Or {
            left: Box::new(self.clone()),
            right: Box::new(other),
        }
    }
}

impl Into<topk_protos::v1::data::TextExpr> for TextExpr {
    fn into(self) -> topk_protos::v1::data::TextExpr {
        match self {
            TextExpr::Terms { all, terms } => topk_protos::v1::data::TextExpr::terms(
                all,
                terms.into_iter().map(|t| t.into()).collect(),
            ),
            TextExpr::And { left, right } => {
                topk_protos::v1::data::TextExpr::and((*left).into(), (*right).into())
            }
            TextExpr::Or { left, right } => {
                topk_protos::v1::data::TextExpr::or((*left).into(), (*right).into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub token: String,
    pub field: Option<String>,
    pub weight: f32,
}

impl Into<topk_protos::v1::data::text_expr::Term> for Term {
    fn into(self) -> topk_protos::v1::data::text_expr::Term {
        topk_protos::v1::data::text_expr::Term {
            token: self.token,
            field: self.field,
            weight: self.weight,
        }
    }
}
