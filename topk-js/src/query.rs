use std::collections::HashMap;

use crate::{
  field::Field, filter_expr::FilterExpression, logical_expr::LogicalExpression,
  select_expr::SelectExpression,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
#[derive(Debug, Clone)]
pub enum Stage {
  Select {
    exprs: HashMap<String, SelectExpression>,
  },
  Filter {
    expr: FilterExpression,
  },
  TopK {
    expr: LogicalExpression,
    k: i32,
    asc: bool,
  },
  Count,
  Rerank {
    model: Option<String>,
    query: Option<String>,
    fields: Vec<String>,
    topk_multiple: Option<u32>,
  },
}

#[napi]
#[derive(Debug)]
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
  pub fn filter(&self, expr: FilterExpression) -> Query {
    let mut new_query = Query {
      stages: self.stages.clone(),
    };

    new_query.stages.push(Stage::Filter { expr });

    new_query
  }

  #[napi(js_name = "top_k")]
  pub fn top_k(&self, expr: Field, k: i32, asc: Option<bool>) -> Query {
    let mut new_query = Query {
      stages: self.stages.clone(),
    };

    new_query.stages.push(Stage::TopK {
      expr: expr.get_expr(),
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

  #[napi(getter)]
  pub fn get_stages(&self) -> Vec<Stage> {
    self.stages.clone()
  }
}

#[napi]
pub fn select(exprs: Vec<Field>) -> Query {
  let mut field_map: HashMap<String, SelectExpression> = HashMap::new();

  for field in &exprs {
    let expr = field.get_expr();

    match &expr {
      LogicalExpression::Field { name } => {
        field_map.insert(name.clone(), SelectExpression::Logical { expr });
      }
      LogicalExpression::Binary {
        left,
        op: _,
        right: _,
      } => {
        if let LogicalExpression::Field { name } = left.as_ref() {
          field_map.insert(name.clone(), SelectExpression::Logical { expr });
        }
      }
      _ => {}
    }
  }

  let stage = Stage::Select { exprs: field_map };

  let stages = vec![stage];

  Query::create(stages)
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
        Stage::Filter { expr } => {
          topk_protos::v1::data::stage::Stage::Filter(topk_protos::v1::data::stage::FilterStage {
            expr: Some(expr.into()),
          })
        }
        Stage::Select { exprs } => {
          topk_protos::v1::data::stage::Stage::Select(topk_protos::v1::data::stage::SelectStage {
            exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
          })
        }
        Stage::TopK { expr, k, asc } => {
          topk_protos::v1::data::stage::Stage::TopK(topk_protos::v1::data::stage::TopKStage {
            expr: Some(expr.into()),
            k: k as u64,
            asc,
          })
        }
        Stage::Count => {
          topk_protos::v1::data::stage::Stage::Count(topk_protos::v1::data::stage::CountStage {})
        }
        Stage::Rerank {
          model,
          query,
          fields,
          topk_multiple,
        } => {
          topk_protos::v1::data::stage::Stage::Rerank(topk_protos::v1::data::stage::RerankStage {
            model,
            query,
            fields,
            topk_multiple,
          })
        }
      }),
    }
  }
}

impl FromNapiValue for Query {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
  ) -> Result<Self, napi::Status> {
    let object = Object::from_napi_value(env, value)?;

    let stages: Option<Vec<Stage>> = object.get("stages".into())?;

    match stages {
      Some(stages) => Ok(Self { stages: stages }),
      None => {
        println!("Received stages: None");
        Err(napi::Error::new(
          napi::Status::GenericFailure,
          "Received stages: None",
        ))
      }
    }
  }
}
