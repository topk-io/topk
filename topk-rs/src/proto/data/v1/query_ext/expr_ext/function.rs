use crate::proto::{
    data::v1::{SparseVector, Value},
    v1::data::{function_expr, FunctionExpr, LogicalExpr},
};

impl FunctionExpr {
    pub fn vector_distance(
        field: impl Into<String>,
        query: impl Into<Value>,
        skip_refine: bool,
    ) -> Self {
        FunctionExpr {
            func: Some(function_expr::Func::VectorDistance(
                function_expr::VectorDistance {
                    field: field.into(),
                    query: Some(query.into()),
                    skip_refine,
                    #[allow(deprecated)]
                    dense_query: None,
                    #[allow(deprecated)]
                    sparse_query: None,
                },
            )),
        }
    }

    pub fn multi_vector_distance(
        field: impl Into<String>,
        query: impl Into<Value>,
        candidates: Option<u32>,
    ) -> Self {
        FunctionExpr {
            func: Some(function_expr::Func::MultiVectorDistance(
                function_expr::MultiVectorDistance {
                    field: field.into(),
                    query: Some(query.into()),
                    candidates,
                    smve: None,
                },
            )),
        }
    }

    pub fn bm25_score(b: Option<f32>, k1: Option<f32>) -> Self {
        if let Some(b) = b {
            if b < 0.0 || b > 1.0 {
                panic!("b must be between 0.0 and 1.0");
            }
        }
        if let Some(k1) = k1 {
            if k1 < 0.0 {
                panic!("k1 must be >= 0.0");
            }
        }
        FunctionExpr {
            func: Some(function_expr::Func::Bm25Score(function_expr::Bm25Score {
                b,
                k1,
            })),
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

    pub fn skip_refine(mut self, skip_refine: bool) -> Self {
        if let Some(function_expr::Func::VectorDistance(vector_distance)) = &mut self.func {
            vector_distance.skip_refine = skip_refine;
        }
        self
    }

    pub fn with_smve(mut self, field: impl Into<String>, smve: impl Into<SparseVector>) -> Self {
        if let Some(function_expr::Func::MultiVectorDistance(multi_vector_distance)) =
            &mut self.func
        {
            multi_vector_distance.smve = Some(function_expr::multi_vector_distance::QuerySmve {
                field: field.into(),
                smve: Some(smve.into()),
            });
        }
        self
    }
}

macro_rules! lift {
    ($($fn:ident($($arg:ident: $ty:ty),*)),* $(,)?) => {
        impl FunctionExpr {
            $(pub fn $fn(self, $($arg: $ty),*) -> LogicalExpr {
                LogicalExpr::function(self).$fn($($arg),*)
            })*
        }
    };
}

lift!(
    // Comparison operators
    eq(rhs: impl Into<LogicalExpr>),
    neq(rhs: impl Into<LogicalExpr>),
    lt(rhs: impl Into<LogicalExpr>),
    lte(rhs: impl Into<LogicalExpr>),
    gt(rhs: impl Into<LogicalExpr>),
    gte(rhs: impl Into<LogicalExpr>),
    // Arithmetic operators
    add(rhs: impl Into<LogicalExpr>),
    sub(rhs: impl Into<LogicalExpr>),
    mul(rhs: impl Into<LogicalExpr>),
    div(rhs: impl Into<LogicalExpr>),
    min(rhs: impl Into<LogicalExpr>),
    max(rhs: impl Into<LogicalExpr>),
    coalesce(rhs: impl Into<LogicalExpr>),
    // Unary operators
    is_null(),
    is_not_null(),
    abs(),
    ln(),
    exp(),
    sqrt(),
    square(),
    // Ternary operators
    choose(x: impl Into<LogicalExpr>, y: impl Into<LogicalExpr>),
    boost(condition: impl Into<LogicalExpr>, boost: impl Into<Value>),
);
