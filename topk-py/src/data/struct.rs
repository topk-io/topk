use std::collections::HashMap;

use pyo3::{pyclass, pymethods};

use crate::data::value::Value;

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub(crate) fields: HashMap<String, Value>,
}

#[pymethods]
impl Struct {
    fn __str__(&self) -> String {
        format!("Struct({:?})", self.fields)
    }
}
