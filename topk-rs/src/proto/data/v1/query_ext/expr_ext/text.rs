use crate::proto::{
    data::v1::text_expr::Term,
    v1::data::{text_expr, TextExpr},
};

impl TextExpr {
    pub fn terms(all: bool, terms: Vec<Term>) -> Self {
        TextExpr {
            expr: Some(text_expr::Expr::Terms(text_expr::TextTermsExpr {
                all,
                terms,
            })),
        }
    }

    pub fn and(self, right: impl Into<TextExpr>) -> Self {
        TextExpr {
            expr: Some(text_expr::Expr::And(Box::new(text_expr::TextAndExpr {
                left: Some(Box::new(self)),
                right: Some(Box::new(right.into())),
            }))),
        }
    }

    pub fn or(self, right: impl Into<TextExpr>) -> Self {
        TextExpr {
            expr: Some(text_expr::Expr::Or(Box::new(text_expr::TextOrExpr {
                left: Some(Box::new(self)),
                right: Some(Box::new(right.into())),
            }))),
        }
    }
}
