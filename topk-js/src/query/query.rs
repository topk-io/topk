use std::collections::HashMap;

use super::stage::Stage;
use crate::expr::{
    filter::FilterExpression,
    logical::LogicalExpression,
    select::SelectExpression,
    text::{Term, TextExpression},
};
use napi_derive::napi;

/// @internal
/// @hideconstructor
/// A query object that represents a sequence of query stages.
///
/// Queries are built by chaining together different stages like select, filter, topk, etc.
/// Each stage performs a specific operation on the data.
#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct Query {
    pub(crate) stages: Vec<Stage>,
}

#[napi(namespace = "query")]
impl Query {
    /// Adds a filter stage to the query.
    #[napi]
    pub fn filter(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | TextExpression")] expr: FilterExpression,
    ) -> Query {
        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        new_query.stages.push(Stage::Filter { expr });

        new_query
    }

    /// Adds a select stage to the query.
    #[napi]
    pub fn select(
        &self,
        #[napi(ts_arg_type = "Record<string, LogicalExpression | FunctionExpression>")]
        exprs: HashMap<String, SelectExpression>,
    ) -> Query {
        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        new_query.stages.push(Stage::Select {
            exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
        });

        new_query
    }

    /// Adds a top-k stage to the query.
    #[napi]
    pub fn topk(&self, expr: &LogicalExpression, k: i32, asc: Option<bool>) -> Query {
        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        new_query.stages.push(Stage::TopK {
            expr: expr.clone(),
            k,
            asc: asc.unwrap_or(false),
        });

        new_query
    }

    /// Adds a count stage to the query.
    #[napi]
    pub fn count(&self) -> Query {
        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        new_query.stages.push(Stage::Count {});

        new_query
    }

    /// Adds a rerank stage to the query.
    #[napi]
    pub fn rerank(&self, options: Option<RerankOptions>) -> Query {
        let options = options.unwrap_or_default();

        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        new_query.stages.push(Stage::Rerank {
            model: options.model,
            query: options.query,
            fields: options.fields.unwrap_or_default(),
            topk_multiple: options.topk_multiple,
        });

        new_query
    }
}

/// Options for rerank stages.
///
/// This struct contains configuration options for reranking results,
/// including the model, query, and fields to use.
#[napi(object)]
#[derive(Default)]
pub struct RerankOptions {
    /// The reranking model to use
    pub model: Option<String>,
    /// The query text for reranking
    pub query: Option<String>,
    /// Fields to include in reranking
    pub fields: Option<Vec<String>>,
    /// Multiple of top-k to consider for reranking
    pub topk_multiple: Option<u32>,
}

/// Options for text matching.
///
/// This struct contains configuration options for text matching operations,
/// including field specification, weight, and matching behavior.
#[napi(object, namespace = "query")]
#[derive(Default)]
pub struct MatchOptions {
    /// Field to match against
    pub field: Option<String>,
    /// Weight for the match
    pub weight: Option<f64>,
    /// Whether to match all terms
    pub all: Option<bool>,
}

/// Creates a text match expression.
#[napi(js_name = "match", namespace = "query")]
pub fn match_(token: String, options: Option<MatchOptions>) -> TextExpression {
    let options = options.unwrap_or_default();

    TextExpression::terms(
        options.all.unwrap_or(false),
        vec![Term {
            token,
            field: options.field,
            weight: options.weight.unwrap_or(1.0),
        }],
    )
}

impl From<Query> for topk_rs::proto::v1::data::Query {
    fn from(query: Query) -> Self {
        topk_rs::proto::v1::data::Query::new(
            query.stages.into_iter().map(|stage| stage.into()).collect(),
        )
    }
}
