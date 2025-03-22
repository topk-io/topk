use std::collections::HashMap;

use crate::select_expr::SelectExpression;
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub enum Stage {
  Select {
    exprs: HashMap<String, SelectExpression>,
  },
  // Filter {
  //   expr: FilterExpression,
  // },
  // TopK {
  //   expr: LogicalExpression,
  //   k: u64,
  //   asc: bool,
  // },
  Count {},
  Rerank {
    model: Option<String>,
    query: Option<String>,
    fields: Vec<String>,
    topk_multiple: Option<u32>,
  },
}

#[napi(object)]
pub struct Query {
  pub stages: Vec<Stage>,
}

impl Query {
  pub fn new() -> Self {
    Self { stages: vec![] }
  }

  pub fn select(&mut self, exprs: HashMap<String, SelectExpression>) {
    self.stages.push(Stage::Select { exprs });
  }
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
        Stage::Select { exprs } => {
          topk_protos::v1::data::stage::Stage::Select(topk_protos::v1::data::stage::SelectStage {
            exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
          })
        }
        Stage::Count {} => {
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
