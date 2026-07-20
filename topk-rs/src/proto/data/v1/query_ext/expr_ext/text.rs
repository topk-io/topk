use crate::proto::{
    data::v1::text_expr::Term,
    v1::data::{text_expr, TextExpr},
};

impl TextExpr {
    pub fn terms(all: bool, terms: Vec<Term>) -> Self {
        TextExpr {
            expr: Some(text_expr::Expr::Terms(text_expr::TextTermsExpr {
                all,
                should: false,
                terms,
            })),
        }
    }

    pub fn should(terms: Vec<Term>) -> Self {
        TextExpr {
            expr: Some(text_expr::Expr::Terms(text_expr::TextTermsExpr {
                all: false,
                should: true,
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

    /// Scales every term weight in the expression by `factor`.
    pub fn boost(mut self, factor: f32) -> Self {
        let (left, right) = match self.expr.as_mut() {
            Some(text_expr::Expr::Terms(terms)) => {
                for term in &mut terms.terms {
                    term.weight *= factor;
                }
                return self;
            }
            Some(text_expr::Expr::And(expr)) => (&mut expr.left, &mut expr.right),
            Some(text_expr::Expr::Or(expr)) => (&mut expr.left, &mut expr.right),
            None => return self,
        };

        for side in [left, right] {
            if let Some(boxed) = side.take() {
                *side = Some(Box::new(boxed.boost(factor)));
            }
        }

        self
    }
}
