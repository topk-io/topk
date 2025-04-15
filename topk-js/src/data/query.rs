use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::{
    filter_expr::FilterExpressionUnion,
    logical_expr::{LogicalExpression, LogicalExpressionUnion},
    select_expr::SelectExpression,
    stage::Stage,
    text_expr::{Term, TextExpression, TextExpressionUnion},
    value::Value,
};

#[napi]
#[derive(Debug, Clone)]
pub struct Query {
    stages: Vec<Stage>,
}

#[napi]
impl Query {
    #[napi(factory)]
    pub fn create(stages: Vec<Stage>) -> Query {
        Self { stages }
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

    #[napi(js_name = "top_k")]
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
) -> Result<Query> {
    let stage = Stage::Select {
        exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
    };

    let stages = vec![stage];

    Ok(Query { stages })
}

#[napi(namespace = "query")]
pub fn field(name: String) -> LogicalExpression {
    LogicalExpression::create(LogicalExpressionUnion::Field { name })
}

#[napi(namespace = "query")]
pub fn literal(value: Value) -> LogicalExpression {
    LogicalExpression::create(LogicalExpressionUnion::Literal { value })
}

impl From<Query> for topk_protos::v1::data::Query {
    fn from(query: Query) -> Self {
        topk_protos::v1::data::Query {
            stages: query.stages.into_iter().map(|stage| stage.into()).collect(),
        }
    }
}

impl From<Stage> for topk_protos::v1::data::Stage {
    fn from(stage: Stage) -> Self {
        topk_protos::v1::data::Stage {
            stage: Some(match stage {
                Stage::Filter { expr } => topk_protos::v1::data::stage::Stage::Filter(
                    topk_protos::v1::data::stage::FilterStage {
                        expr: Some(expr.into()),
                    },
                ),
                Stage::Select { exprs } => topk_protos::v1::data::stage::Stage::Select(
                    topk_protos::v1::data::stage::SelectStage {
                        exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
                    },
                ),
                Stage::TopK { expr, k, asc } => topk_protos::v1::data::stage::Stage::TopK(
                    topk_protos::v1::data::stage::TopKStage {
                        expr: Some(expr.into()),
                        k: k as u64,
                        asc,
                    },
                ),
                Stage::Count => topk_protos::v1::data::stage::Stage::Count(
                    topk_protos::v1::data::stage::CountStage {},
                ),
                Stage::Rerank {
                    model,
                    query,
                    fields,
                    topk_multiple,
                } => topk_protos::v1::data::stage::Stage::Rerank(
                    topk_protos::v1::data::stage::RerankStage {
                        model,
                        query,
                        fields,
                        topk_multiple,
                    },
                ),
            }),
        }
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
