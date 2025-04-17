use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::{
    filter_expr::FilterExpressionUnion,
    logical_expr::{LogicalExpression, LogicalExpressionUnion},
    scalar::Scalar,
    select_expr::SelectExpression,
    stage::Stage,
    value::Value,
};

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
    pub fn top_k(&self, expr: LogicalExpression, k: i32, asc: Option<bool>) -> Query {
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
    pub fn rerank(
        &self,
        model: Option<String>,
        query: Option<String>,
        fields: Option<Vec<String>>,
        topk_multiple: Option<u32>,
    ) -> Query {
        let mut new_query = Query {
            stages: self.stages.clone(),
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
pub fn field(name: String) -> LogicalExpression {
    LogicalExpression::create(LogicalExpressionUnion::Field { name })
}

#[napi(namespace = "query")]
pub fn literal(
    #[napi(ts_arg_type = "number | string | boolean")] value: Scalar,
) -> LogicalExpression {
    LogicalExpression::create(LogicalExpressionUnion::Literal { value })
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
