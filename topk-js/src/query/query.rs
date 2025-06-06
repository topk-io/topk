use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{
    data::{napi_box::NapiBox, scalar::Scalar},
    expr::{
        filter::FilterExpressionUnion,
        logical::{LogicalExpression, LogicalExpressionUnion, UnaryOperator},
        select::SelectExpression,
        text::{Term, TextExpression, TextExpressionUnion},
    },
};

use super::stage::Stage;

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct Query {
    stages: Vec<Stage>,
}

#[napi(namespace = "query")]
impl Query {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { stages: vec![] }
    }

    #[napi(factory)]
    pub fn select(
        &self,
        #[napi(ts_arg_type = "Record<string, LogicalExpression | FunctionExpression>")]
        exprs: HashMap<String, SelectExpression>,
    ) -> Self {
        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        let stage = Stage::Select {
            exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
        };

        new_query.stages.push(stage);

        new_query
    }

    #[napi]
    pub fn filter(
        &self,
        #[napi(ts_arg_type = "LogicalExpression | TextExpression")] expr: FilterExpressionUnion,
    ) -> Query {
        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        new_query.stages.push(Stage::Filter { expr });

        new_query
    }

    #[napi]
    pub fn topk(&self, expr: LogicalExpression, k: i32, asc: Option<bool>) -> Query {
        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        new_query.stages.push(Stage::TopK {
            expr,
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
        let mut new_query = Query {
            stages: self.stages.clone(),
        };

        let (model, query, fields, topk_multiple) = match options {
            Some(options) => (
                options.model,
                options.query,
                options.fields,
                options.topk_multiple,
            ),
            None => (None, None, None, None),
        };

        new_query.stages.push(Stage::Rerank {
            model,
            query,
            fields: fields.unwrap_or_default(),
            topk_multiple,
        });

        new_query
    }
}

#[napi(object)]
pub struct RerankOptions {
    pub model: Option<String>,
    pub query: Option<String>,
    pub fields: Option<Vec<String>>,
    pub topk_multiple: Option<u32>,
}

#[napi(namespace = "query")]
pub fn select(
    #[napi(ts_arg_type = "Record<string, LogicalExpression | FunctionExpression>")] exprs: HashMap<
        String,
        SelectExpression,
    >,
) -> Query {
    let stage = Stage::Select {
        exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
    };

    let stages = vec![stage];

    Query { stages }
}

#[napi(namespace = "query")]
pub fn filter(
    #[napi(ts_arg_type = "LogicalExpression | TextExpression")] expr: FilterExpressionUnion,
) -> Query {
    let stage = Stage::Filter { expr };

    Query {
        stages: vec![stage],
    }
}

#[napi(namespace = "query")]
pub fn field(name: String) -> LogicalExpression {
    LogicalExpression::create(LogicalExpressionUnion::Field { name })
}

#[napi(namespace = "query")]
pub fn literal(
    #[napi(ts_arg_type = "number | string | boolean")] value: Scalar,
) -> LogicalExpression {
    LogicalExpression::create(LogicalExpressionUnion::Literal { value })
}

#[napi(object)]
pub struct MatchOptions {
    pub field: Option<String>,
    pub weight: Option<f64>,
    pub all: Option<bool>,
}

#[napi(js_name = "match", namespace = "query")]
pub fn match_(token: String, options: Option<MatchOptions>) -> TextExpression {
    let options = match options {
        Some(options) => options,
        None => MatchOptions {
            field: None,
            weight: None,
            all: None,
        },
    };

    TextExpression::create(TextExpressionUnion::Terms {
        all: options.all.unwrap_or(false),
        terms: vec![Term {
            token,
            field: options.field,
            weight: options.weight.unwrap_or(1.0),
        }],
    })
}

#[napi(js_name = "not", namespace = "query")]
pub fn not(expr: LogicalExpression) -> LogicalExpression {
    LogicalExpression::create(LogicalExpressionUnion::Unary {
        op: UnaryOperator::Not,
        expr: NapiBox(Box::new(expr.get_expr())),
    })
}

impl From<Query> for topk_rs::query::Query {
    fn from(query: Query) -> Self {
        topk_rs::query::Query::new(query.stages.into_iter().map(|stage| stage.into()).collect())
    }
}

impl FromNapiValue for Query {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        let query = Query::from_napi_ref(env, value)?;

        let stages: Vec<Stage> = query.stages.clone();

        Ok(Self { stages })
    }
}
