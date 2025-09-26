use pyo3::{pyclass, pymethods, FromPyObject, IntoPyObject};

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub(crate) values: Values,
}

#[derive(Debug, Clone, PartialEq, FromPyObject, IntoPyObject)]
pub enum Values {
    U8(Vec<u8>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    I8(Vec<i8>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    String(Vec<String>),
}

#[pymethods]
impl List {
    fn __str__(&self) -> String {
        match &self.values {
            Values::U8(values) => format!("List(U8({:?}))", values),
            Values::U32(values) => format!("List(U32({:?}))", values),
            Values::U64(values) => format!("List(U64({:?}))", values),
            Values::I8(values) => format!("List(I8({:?}))", values),
            Values::I32(values) => format!("List(I32({:?}))", values),
            Values::I64(values) => format!("List(I64({:?}))", values),
            Values::F32(values) => format!("List(F32({:?}))", values),
            Values::F64(values) => format!("List(F64({:?}))", values),
            Values::String(values) => format!("List(String({:?}))", values),
        }
    }
}
