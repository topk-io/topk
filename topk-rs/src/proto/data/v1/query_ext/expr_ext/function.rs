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

// Lift into `LogicalExpr` so score functions compose directly with
// comparisons and arithmetic, e.g. `fns::vector_distance(..).gt(0.5)`.
macro_rules! lift {
    ($($fn:ident),*) => {
        impl FunctionExpr {
            $(pub fn $fn(self, rhs: impl Into<LogicalExpr>) -> LogicalExpr {
                LogicalExpr::function(self).$fn(rhs)
            })*
        }
    };
}

macro_rules! lift_unary {
    ($($fn:ident),*) => {
        impl FunctionExpr {
            $(pub fn $fn(self) -> LogicalExpr {
                LogicalExpr::function(self).$fn()
            })*
        }
    };
}

lift!(gt, gte, lt, lte, eq, neq, add, sub, mul, div, min, max, coalesce);
lift_unary!(is_null, is_not_null, abs, ln, exp, sqrt, square);

impl FunctionExpr {
    pub fn choose(self, x: impl Into<LogicalExpr>, y: impl Into<LogicalExpr>) -> LogicalExpr {
        LogicalExpr::function(self).choose(x, y)
    }

    pub fn boost(
        self,
        condition: impl Into<LogicalExpr>,
        boost: impl Into<crate::proto::v1::data::Value>,
    ) -> LogicalExpr {
        LogicalExpr::function(self).boost(condition, boost)
    }
}
