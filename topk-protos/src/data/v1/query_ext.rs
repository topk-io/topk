use logical_expr::{binary_op, unary_op, BinaryOp, UnaryOp};
use text_expr::Term;

use super::*;

impl Query {
    pub fn new(stages: Vec<Stage>) -> Self {
        Query { stages }
    }
}

impl Stage {
    pub fn select(
        exprs: std::collections::HashMap<String, impl Into<stage::select_stage::SelectExpr>>,
    ) -> Self {
        Stage {
            stage: Some(stage::Stage::Select(stage::SelectStage {
                exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
            })),
        }
    }

    pub fn filter(expr: impl Into<stage::filter_stage::FilterExpr>) -> Self {
        Stage {
            stage: Some(stage::Stage::Filter(stage::FilterStage {
                expr: Some(expr.into()),
            })),
        }
    }

    pub fn topk(expr: LogicalExpr, k: u64, asc: bool) -> Self {
        Stage {
            stage: Some(stage::Stage::TopK(stage::TopKStage {
                expr: Some(expr),
                k,
                asc,
            })),
        }
    }

    pub fn count() -> Self {
        Stage {
            stage: Some(stage::Stage::Count(stage::CountStage {})),
        }
    }

    pub fn rerank(
        model: Option<String>,
        query: Option<String>,
        fields: Vec<String>,
        topk_multiple: Option<u32>,
    ) -> Self {
        Stage {
            stage: Some(stage::Stage::Rerank(stage::RerankStage {
                model,
                query,
                fields,
                topk_multiple,
            })),
        }
    }
}

impl stage::select_stage::SelectExpr {
    pub fn logical(expr: LogicalExpr) -> Self {
        stage::select_stage::SelectExpr {
            expr: Some(stage::select_stage::select_expr::Expr::LogicalExpr(expr)),
        }
    }

    pub fn function(func: FunctionExpr) -> Self {
        stage::select_stage::SelectExpr {
            expr: Some(stage::select_stage::select_expr::Expr::FunctionExpr(func)),
        }
    }
}

impl LogicalExpr {
    pub fn field(name: impl Into<String>) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::Field(name.into())),
        }
    }

    pub fn literal(value: Value) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::Literal(value)),
        }
    }

    pub fn unary(op: unary_op::Op, expr: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::UnaryOp(Box::new(
                logical_expr::UnaryOp {
                    op: op as i32,
                    expr: Some(Box::new(expr)),
                },
            ))),
        }
    }

    pub fn binary(op: binary_op::Op, left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: op as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn and(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::And as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn or(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Or as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn eq(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Eq as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn neq(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Neq as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn lt(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Lt as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn lte(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Lte as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn gt(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Gt as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn gte(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Gte as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn starts_with(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::StartsWith as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn contains(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Contains as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn add(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Add as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn mul(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Mul as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn div(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Div as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }

    pub fn sub(left: LogicalExpr, right: LogicalExpr) -> Self {
        LogicalExpr {
            expr: Some(logical_expr::Expr::BinaryOp(Box::new(
                logical_expr::BinaryOp {
                    op: logical_expr::binary_op::Op::Sub as i32,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                },
            ))),
        }
    }
}

impl UnaryOp {
    pub fn not(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::Not as i32,
            expr: Some(Box::new(expr)),
        }
    }

    pub fn is_null(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::IsNull as i32,
            expr: Some(Box::new(expr)),
        }
    }

    pub fn is_not_null(expr: LogicalExpr) -> Self {
        UnaryOp {
            op: logical_expr::unary_op::Op::IsNotNull as i32,
            expr: Some(Box::new(expr)),
        }
    }
}

impl BinaryOp {
    pub fn and(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::And as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn or(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Or as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn eq(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Eq as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn neq(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Neq as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn lt(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Lt as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn lte(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Lte as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn gt(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Gt as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn gte(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Gte as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn add(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Add as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn mul(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Mul as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn div(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Div as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    pub fn sub(left: LogicalExpr, right: LogicalExpr) -> Self {
        BinaryOp {
            op: logical_expr::binary_op::Op::Sub as i32,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}

impl stage::filter_stage::FilterExpr {
    pub fn logical(expr: LogicalExpr) -> Self {
        stage::filter_stage::FilterExpr {
            expr: Some(stage::filter_stage::filter_expr::Expr::LogicalExpr(expr)),
        }
    }

    pub fn text(expr: TextExpr) -> Self {
        stage::filter_stage::FilterExpr {
            expr: Some(stage::filter_stage::filter_expr::Expr::TextExpr(expr)),
        }
    }
}

impl FunctionExpr {
    pub fn vector_distance(field: impl Into<String>, query: Vector) -> Self {
        FunctionExpr {
            func: Some(function_expr::Func::VectorDistance(
                function_expr::VectorDistance {
                    field: field.into(),
                    query: Some(query),
                },
            )),
        }
    }

    pub fn bm25_score() -> Self {
        FunctionExpr {
            func: Some(function_expr::Func::Bm25Score(function_expr::Bm25Score {})),
        }
    }

    pub fn semantic_similarity(field: impl Into<String>, query: impl Into<String>) -> Self {
        FunctionExpr {
            func: Some(function_expr::Func::SemanticSimilarity(
                function_expr::SemanticSimilarity {
                    field: field.into(),
                    query: query.into(),
                },
            )),
        }
    }
}

impl TextExpr {
    pub fn terms(all: bool, terms: Vec<Term>) -> Self {
        TextExpr {
            expr: Some(text_expr::Expr::Terms(text_expr::TextTermsExpr {
                all,
                terms,
            })),
        }
    }

    pub fn and(left: TextExpr, right: TextExpr) -> Self {
        TextExpr {
            expr: Some(text_expr::Expr::And(Box::new(text_expr::TextAndExpr {
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
            }))),
        }
    }

    pub fn or(left: TextExpr, right: TextExpr) -> Self {
        TextExpr {
            expr: Some(text_expr::Expr::Or(Box::new(text_expr::TextOrExpr {
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
            }))),
        }
    }
}
