
#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use napi::bindgen_prelude::*;
use std::collections::HashMap;

#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
  a + b
}

#[napi(object)]
struct Foo {
	pub name: String,
	pub version: String,
	pub dependencies: Option<HashMap<String, String>>,
	pub dev_dependencies: Option<HashMap<String, String>>,
}

#[napi]
pub fn topk() -> Foo {
  Foo {
    name: "topk".to_string(),
    version: "1.0.0".to_string(),
    dependencies: None,
    dev_dependencies: None,
  }
}

#[napi]
pub enum Kind {
  Dog,
  Cat,
  Duck,
}

#[napi(constructor)]
struct Animal {
  pub name: String,
  pub kind: u32,
}

#[napi]
impl Animal {
  #[napi]
  pub fn change_name(&mut self, new_name: String) {
    self.name = new_name;
  }
}