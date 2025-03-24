use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub enum LogicalExpression {
  Null,
  Field {
    name: String,
  },
  Literal {
    value: String,
  },
  And {
    #[napi(ts_type = "LogicalExpression")]
    left: MyBox<LogicalExpression>,
    #[napi(ts_type = "LogicalExpression")]
    right: MyBox<LogicalExpression>,
  },
  Or {
    #[napi(ts_type = "LogicalExpression")]
    left: MyBox<LogicalExpression>,
    #[napi(ts_type = "LogicalExpression")]
    right: MyBox<LogicalExpression>,
  },
}

#[derive(Debug)]
pub struct MyBox<T>(pub Box<T>);

impl<T> FromNapiValue for MyBox<T>
where
  T: FromNapiValue,
{
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    napi_val: napi::sys::napi_value,
  ) -> napi::Result<Self> {
    T::from_napi_value(env, napi_val).map(|v| Self(Box::new(v)))
  }
}

impl<T> ToNapiValue for MyBox<T>
where
  T: ToNapiValue,
{
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> napi::Result<napi::sys::napi_value> {
    ToNapiValue::to_napi_value(env, *val.0)
  }
}
