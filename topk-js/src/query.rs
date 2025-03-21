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

#[napi]
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

fn testiek() {
  let mut query = Query::new();
  query.select(HashMap::new());
}
