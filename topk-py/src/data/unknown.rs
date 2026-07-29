use pyo3::prelude::*;

pub const UNSUPPORTED: &str = "not supported by this version of topk-sdk, upgrade to use it";

#[pyclass(frozen, eq)]
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownValue {}

#[pymethods]
impl UnknownValue {
    #[new]
    fn new() -> Self {
        Self {}
    }

    fn __repr__(&self) -> String {
        format!("UnknownValue({UNSUPPORTED})")
    }
}
