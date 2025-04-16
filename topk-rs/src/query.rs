pub use crate::data::stage::Stage;
use crate::data::{
    filter_expr::FilterExpr,
    logical_expr::LogicalExpr,
    scalar::Scalar,
    select_expr::SelectExpr,
    text_expr::{Term, TextExpr},
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Query {
    stages: Vec<Stage>,
}

impl Query {
    pub fn new(stages: Vec<Stage>) -> Self {
        Self { stages }
    }

    pub fn select<S, E>(&self, exprs: impl IntoIterator<Item = (S, E)>) -> Self
    where
        S: Into<String>,
        E: Into<SelectExpr>,
    {
        let mut query = self.clone();
        query.stages.push(Stage::Select {
            exprs: exprs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        });
        query
    }

    pub fn filter(&self, expr: impl Into<FilterExpr>) -> Self {
        let mut query = self.clone();
        query.stages.push(Stage::Filter {
            expr: expr.into().into(),
        });
        query
    }

    pub fn top_k(&self, expr: impl Into<LogicalExpr>, limit: u64, asc: bool) -> Self {
        let mut query = self.clone();
        query.stages.push(Stage::TopK {
            expr: expr.into().into(),
            k: limit,
            asc,
        });
        query
    }

    pub fn count(&self) -> Self {
        let mut query = self.clone();
        query.stages.push(Stage::Count {});
        query
    }

    pub fn rerank(
        &self,
        model: Option<String>,
        query: Option<String>,
        fields: Vec<String>,
        topk_multiple: Option<u32>,
    ) -> Self {
        let mut new_query = self.clone();
        new_query.stages.push(Stage::Rerank {
            model,
            query,
            fields,
            topk_multiple,
        });
        new_query
    }
}
impl From<Query> for topk_protos::v1::data::Query {
    fn from(query: Query) -> Self {
        Self {
            stages: query.stages.into_iter().map(|s| s.into()).collect(),
        }
    }
}

pub fn select<S, E>(exprs: impl IntoIterator<Item = (S, E)>) -> Query
where
    S: Into<String>,
    E: Into<SelectExpr>,
{
    let exprs: HashMap<String, SelectExpr> = exprs
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();

    Query::new(vec![]).select(exprs)
}

// TODO: `filter` and `top_k` are not exported from python
pub fn filter(expr: impl Into<FilterExpr>) -> Query {
    Query::new(vec![]).filter(expr.into())
}

pub fn top_k(expr: impl Into<LogicalExpr>, limit: u64, asc: bool) -> Query {
    Query::new(vec![]).top_k(expr, limit, asc)
}

pub fn field<S>(name: S) -> LogicalExpr
where
    S: Into<String>,
{
    LogicalExpr::Field { name: name.into() }
}

pub fn literal<V>(value: V) -> LogicalExpr
where
    V: Into<Scalar>,
{
    LogicalExpr::Literal {
        value: value.into(),
    }
}

pub fn r#match(value: &str, field: Option<&str>, weight: Option<f32>) -> TextExpr {
    TextExpr::Terms {
        all: false,
        terms: vec![Term {
            token: value.to_string(),
            field: field.map(|f| f.to_string()),
            weight: weight.unwrap_or(1.0),
        }],
    }
}

pub mod fns {
    use crate::data::function_expr::{FunctionExpr, Vector};

    pub fn bm25_score() -> FunctionExpr {
        FunctionExpr::KeywordScore {}
    }

    pub fn vector_distance(field: &str, query: impl Into<Vector>) -> FunctionExpr {
        FunctionExpr::VectorScore {
            field: field.to_string(),
            query: query.into(),
        }
    }

    pub fn semantic_similarity(field: &str, query: &str) -> FunctionExpr {
        FunctionExpr::SemanticSimilarity {
            field: field.to_string(),
            query: query.to_string(),
        }
    }
}
