use napi_derive::napi;

use crate::data::Value;
use crate::expr::logical::{
    Boolish, Comparable, LogicalExpression, LogicalExpressionUnion, Numeric, Ordered,
};

/// @internal
/// @hideconstructor
#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct FunctionExpression(pub(crate) FunctionExpressionUnion);

#[derive(Debug, Clone)]
pub enum FunctionExpressionUnion {
    KeywordScore {
        b: Option<f32>,
        k1: Option<f32>,
    },
    VectorScore {
        field: String,
        query: Value,
        skip_refine: bool,
    },
    MultiVectorDistance {
        field: String,
        query: Value,
        candidates: Option<u32>,
    },
    SemanticSimilarity {
        field: String,
        query: String,
    },
}

impl From<FunctionExpression> for topk_rs::proto::v1::data::FunctionExpr {
    fn from(expr: FunctionExpression) -> Self {
        match expr.0 {
            FunctionExpressionUnion::KeywordScore { b, k1 } => {
                topk_rs::proto::v1::data::FunctionExpr::bm25_score(b, k1)
            }
            FunctionExpressionUnion::VectorScore {
                field,
                query,
                skip_refine,
            } => topk_rs::proto::v1::data::FunctionExpr::vector_distance(field, query, skip_refine),
            FunctionExpressionUnion::MultiVectorDistance {
                field,
                query,
                candidates,
            } => topk_rs::proto::v1::data::FunctionExpr::multi_vector_distance(
                field, query, candidates,
            ),
            FunctionExpressionUnion::SemanticSimilarity { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}

macro_rules! lift {
    ($($fn:ident($($arg:ident: $ty:ty as $ts:literal),*)),* $(,)?) => {
        #[napi(namespace = "query")]
        impl FunctionExpression {
            $(#[napi]
            pub fn $fn(
                &self,
                $(#[napi(ts_arg_type = $ts)] $arg: $ty),*
            ) -> LogicalExpression {
                self.lifted().$fn($($arg),*)
            })*
        }
    };
}

impl FunctionExpression {
    pub(crate) fn lifted(&self) -> LogicalExpression {
        LogicalExpression {
            expr: LogicalExpressionUnion::Function { expr: self.clone() },
        }
    }
}

lift!(
    // Comparison operators
    eq(other: Comparable as "LogicalExpression | string | number | boolean | null | undefined"),
    ne(other: Comparable as "LogicalExpression | string | number | boolean | null | undefined"),
    lt(other: Ordered as "LogicalExpression | number | string"),
    lte(other: Ordered as "LogicalExpression | number | string"),
    gt(other: Ordered as "LogicalExpression | number | string"),
    gte(other: Ordered as "LogicalExpression | number | string"),
    // Arithmetic operators
    add(other: Numeric as "LogicalExpression | number"),
    sub(other: Numeric as "LogicalExpression | number"),
    mul(other: Numeric as "LogicalExpression | number"),
    div(other: Numeric as "LogicalExpression | number"),
    min(other: Ordered as "LogicalExpression | number | string"),
    max(other: Ordered as "LogicalExpression | number | string"),
    coalesce(other: Numeric as "LogicalExpression | number"),
    // Unary operators
    is_null(),
    is_not_null(),
    abs(),
    ln(),
    exp(),
    sqrt(),
    square(),
    // Ternary operators
    choose(
        x: Comparable as "LogicalExpression | string | number | boolean | null | undefined",
        y: Comparable as "LogicalExpression | string | number | boolean | null | undefined"
    ),
    boost(
        condition: Boolish as "LogicalExpression | boolean",
        boost: Numeric as "LogicalExpression | number"
    ),
);
