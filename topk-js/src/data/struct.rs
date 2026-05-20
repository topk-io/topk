use std::collections::HashMap;

use napi_derive::napi;

use crate::data::Value;

/// @internal
/// @hideconstructor
/// Instances of the `Struct` class are used to represent nested object values in TopK.
/// Usually created using the [`struct()`](https://docs.topk.io/sdk/topk-js/data#struct-2) helper.
#[derive(Debug, Clone, PartialEq)]
#[napi(namespace = "data")]
pub struct Struct {
    pub(crate) fields: HashMap<String, Value>,
}

/// @internal
#[napi(namespace = "data")]
impl Struct {
    /// @ignore
    #[napi]
    pub fn to_string(&self) -> String {
        format!("Struct({:?})", self.fields)
    }
}
