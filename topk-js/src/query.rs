use std::collections::HashMap;

use crate::{logical_expr::LogicalExpression, my_vec::MyVec, select_expr::SelectExpression};
use napi::{bindgen_prelude::*, JsObject, NapiValue};
use napi_derive::napi;

#[napi]
#[derive(Debug, Clone)]
pub enum Stage {
  Select {
    exprs: HashMap<String, SelectExpression>,
  },
  // Filter {
  //   expr: SelectExpression,
  // },
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

#[derive(Debug, Clone)]
pub struct Query {
  pub stages: MyVec<Stage>,
}

// impl Clone for Query {
//   fn clone(&self) -> Self {
//     Query {
//       stages: self.stages.clone(),
//     }
//   }
// }

#[napi(js_name = "Query")]
pub struct JsQuery {
  query: Query,
}

#[napi]
impl JsQuery {
  #[napi(constructor)]
  pub fn new(stages: Vec<Stage>) -> Self {
    JsQuery {
      query: Query {
        stages: MyVec(stages),
      },
    }
  }

  #[napi]
  pub fn select(&mut self, exprs: HashMap<String, SelectExpression>) -> &mut JsQuery {
    self.query.stages.0.push(Stage::Select { exprs });
    self
  }

  #[napi(js_name = "top_k")]
  pub fn top_k(&mut self, expr: LogicalExpression, k: i32, asc: bool) {
    // let mut query = Query { stages: MyVec(vec![]) };
    // query.stages.0.push(Stage::TopK { expr, k, asc });
    // JsQuery { query }
    todo!()
  }

  #[napi(getter)]
  pub fn query(&self) -> napi::Result<Query> {
    Ok(self.query.clone())
  }
}

#[napi]
pub fn select(exprs: HashMap<String, SelectExpression>) -> JsQuery {
  let mut query = JsQuery::new(vec![]);
  query.select(exprs);
  query
}

impl From<Query> for topk_protos::v1::data::Query {
  fn from(query: Query) -> Self {
    topk_protos::v1::data::Query {
      stages: query
        .stages
        .0
        .into_iter()
        .map(|stage| stage.into())
        .collect(),
    }
  }
}

impl From<Stage> for topk_protos::v1::data::Stage {
  fn from(stage: Stage) -> Self {
    topk_protos::v1::data::Stage {
      stage: Some(match stage {
        // Stage::Filter { expr } => {
        //   topk_protos::v1::data::stage::Stage::Filter(topk_protos::v1::data::stage::FilterStage {
        //     expr: Some(expr.into()),
        //   })
        // }
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
      Some(stages) => {
        println!("Received stages: {:?}", stages);
        Ok(Self {
          stages: MyVec(stages),
        })
      }
      None => todo!(),
    }
  }
}

impl ToNapiValue for Query {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> Result<napi::sys::napi_value, napi::Status> {
    let mut obj = std::ptr::null_mut();
    let status = napi::sys::napi_create_object(env, &mut obj);
    if status != napi::sys::Status::napi_ok {
      return Err(napi::Error::new(
        napi::Status::GenericFailure,
        "Failed to create object",
      ));
    }
    let stages = ToNapiValue::to_napi_value(env, val.stages)?;
    let status =
      napi::sys::napi_set_named_property(env, obj, "stages\0".as_ptr() as *const i8, stages);
    if status != napi::sys::Status::napi_ok {
      return Err(napi::Error::new(
        napi::Status::GenericFailure,
        "Failed to set named property",
      ));
    }
    Ok(obj)
  }
}

impl ToNapiValue for MyVec<Stage> {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> Result<napi::sys::napi_value, napi::Status> {
    let mut obj = std::ptr::null_mut();
    let status = napi::sys::napi_create_object(env, &mut obj);
    if status != napi::sys::Status::napi_ok {
      return Err(napi::Error::new(
        napi::Status::GenericFailure,
        "Failed to create object",
      ));
    }
    let stages = ToNapiValue::to_napi_value(env, val.0)?;
    let status =
      napi::sys::napi_set_named_property(env, obj, "stages\0".as_ptr() as *const i8, stages);
    if status != napi::sys::Status::napi_ok {
      return Err(napi::Error::new(
        napi::Status::GenericFailure,
        "Failed to set named property",
      ));
    }
    Ok(obj)
  }
}

impl ToNapiValue for &mut MyVec<Stage> {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> Result<napi::sys::napi_value, napi::Status> {
    todo!()
  }
}

impl FromNapiValue for MyVec<Stage> {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
  ) -> Result<Self, napi::Status> {
    todo!()
  }
}

impl ToNapiValue for &mut JsQuery {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> Result<napi::sys::napi_value, napi::Status> {
    todo!()
  }
}
