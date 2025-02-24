use super::{
    filter_expr::FilterExpressionUnion,
    logical_expr::LogicalExpression,
    select_expr::{SelectExpression, SelectExpressionUnion},
    stage::Stage,
};
use pyo3::{exceptions::PyTypeError, prelude::*, types::PyString};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ConsistencyLevel {
    Indexed,
    Strong,
}

impl<'py> FromPyObject<'py> for ConsistencyLevel {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let obj = ob.as_ref();

        match obj.downcast::<PyString>() {
            Ok(val) => match val.extract::<&str>()? {
                "indexed" => Ok(ConsistencyLevel::Indexed),
                "strong" => Ok(ConsistencyLevel::Strong),
                val => Err(PyTypeError::new_err(format!(
                    "Invalid consistency level `{val}`",
                ))),
            },
            _ => Err(PyTypeError::new_err(format!(
                "Can't convert from {:?} to ConsistencyLevel type",
                obj.get_type().name()
            ))),
        }
    }
}

impl From<ConsistencyLevel> for topk_protos::v1::data::ConsistencyLevel {
    fn from(consistency_level: ConsistencyLevel) -> Self {
        match consistency_level {
            ConsistencyLevel::Indexed => topk_protos::v1::data::ConsistencyLevel::Indexed,
            ConsistencyLevel::Strong => topk_protos::v1::data::ConsistencyLevel::Strong,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Query {
    pub stages: Vec<Stage>,
}

#[pymethods]
impl Query {
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    #[staticmethod]
    pub fn new() -> Self {
        Self { stages: vec![] }
    }

    #[pyo3(signature = (*args, **kwargs))]
    pub fn select(
        &self,
        args: Vec<String>,
        kwargs: Option<HashMap<String, SelectExpressionUnion>>,
    ) -> PyResult<Self> {
        let exprs = {
            let mut exprs = HashMap::new();

            // apply `*args`
            for key in args {
                exprs.insert(
                    key.clone(),
                    SelectExpression::Logical(LogicalExpression::Field { name: key }),
                );
            }

            // apply `**kwargs`
            for (key, value) in kwargs.unwrap_or_default() {
                exprs.insert(
                    key.clone(),
                    match value {
                        SelectExpressionUnion::Logical(expr) => SelectExpression::Logical(expr),
                        SelectExpressionUnion::Function(expr) => SelectExpression::Function(expr),
                    },
                );
            }

            exprs
        };

        Ok(Self {
            stages: [self.stages.clone(), vec![Stage::Select { exprs }]].concat(),
        })
    }

    pub fn filter(&self, expr: FilterExpressionUnion) -> PyResult<Self> {
        Ok(Self {
            stages: [
                self.stages.clone(),
                vec![Stage::Filter { expr: expr.into() }],
            ]
            .concat(),
        })
    }

    #[pyo3(signature = (expr, k, asc=false))]
    pub fn top_k(&self, expr: Py<LogicalExpression>, k: u64, asc: bool) -> PyResult<Self> {
        Ok(Self {
            stages: [
                self.stages.clone(),
                vec![Stage::TopK {
                    expr: expr.get().clone(),
                    k,
                    asc,
                }],
            ]
            .concat(),
        })
    }

    pub fn count(&self) -> PyResult<Self> {
        Ok(Self {
            stages: [self.stages.clone(), vec![Stage::Count {}]].concat(),
        })
    }
}

impl From<Query> for topk_protos::v1::data::Query {
    fn from(query: Query) -> Self {
        Self {
            stages: query.stages.into_iter().map(|s| s.into()).collect(),
        }
    }
}
