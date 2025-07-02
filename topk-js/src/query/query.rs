use super::stage::Stage;
use crate::expr::{
    filter::FilterExpression,
    logical::LogicalExpression,
    text::{Term, TextExpression},
};
use napi_derive::napi;

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct Query {
    pub(crate) stages: Vec<Stage>,
}

#[napi(namespace = "query")]
impl Query {
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

    #[napi]
    pub fn count(&self) -> Query {
        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        new_query.stages.push(Stage::Count {});

        new_query
    }

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

#[napi(object)]
#[derive(Default)]
pub struct RerankOptions {
    pub model: Option<String>,
    pub query: Option<String>,
    pub fields: Option<Vec<String>>,
    pub topk_multiple: Option<u32>,
}

#[napi(object)]
#[derive(Default)]
pub struct MatchOptions {
    pub field: Option<String>,
    pub weight: Option<f64>,
    pub all: Option<bool>,
}

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
