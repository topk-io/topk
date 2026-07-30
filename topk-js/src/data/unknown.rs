use napi_derive::napi;

pub const UNSUPPORTED: &str = "not supported by this version of topk-js, upgrade to use it";

/// @internal
/// Placeholder for a value this version of the SDK cannot represent.
#[derive(Debug, Clone, PartialEq)]
#[napi(namespace = "data")]
pub struct UnknownValue {}

/// @internal
#[napi(namespace = "data")]
impl UnknownValue {
    /// @ignore
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    /// @ignore
    #[napi]
    pub fn to_string(&self) -> String {
        format!("UnknownValue({UNSUPPORTED})")
    }
}
