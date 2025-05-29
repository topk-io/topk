use crate::expr::filter::FilterExprUnion;
use crate::expr::logical::LogicalExpr;
use crate::expr::select::{SelectExpr, SelectExprUnion};
use pyo3::{exceptions::PyTypeError, prelude::*, types::PyString};
use std::collections::HashMap;

use super::stage::Stage;

#[derive(Debug, Clone)]
pub enum ConsistencyLevel {
    Indexed,
    Strong,
}

impl<'py> FromPyObject<'py> for ConsistencyLevel {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
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
        kwargs: Option<HashMap<String, SelectExprUnion>>,
    ) -> PyResult<Self> {
        let exprs = {
            let mut exprs = HashMap::new();

            // apply `*args`
            for key in args {
                exprs.insert(
                    key.clone(),
                    SelectExpr::Logical(LogicalExpr::Field { name: key }),
                );
            }

            // apply `**kwargs`
            for (key, value) in kwargs.unwrap_or_default() {
                exprs.insert(
                    key.clone(),
                    match value {
                        SelectExprUnion::Logical(expr) => SelectExpr::Logical(expr),
                        SelectExprUnion::Function(expr) => SelectExpr::Function(expr),
                    },
                );
            }

            exprs
        };

        Ok(Self {
            stages: [self.stages.clone(), vec![Stage::Select { exprs }]].concat(),
        })
    }

    pub fn filter(&self, expr: FilterExprUnion) -> PyResult<Self> {
        Ok(Self {
            stages: [
                self.stages.clone(),
                vec![Stage::Filter { expr: expr.into() }],
            ]
            .concat(),
        })
    }

    #[pyo3(signature = (expr, k, asc=false))]
    pub fn topk(&self, expr: LogicalExpr, k: u64, asc: bool) -> PyResult<Self> {
        Ok(Self {
            stages: [
                self.stages.clone(),
                vec![Stage::TopK {
                    expr: expr.into(),
                    k,
                    asc,
                }],
            ]
            .concat(),
        })
    }

    #[pyo3(signature = (model=None, query=None, fields=vec![], topk_multiple=None))]
    pub fn rerank(
        &self,
        model: Option<String>,
        query: Option<String>,
        fields: Vec<String>,
        topk_multiple: Option<u32>,
    ) -> PyResult<Self> {
        Ok(Self {
            stages: [
                self.stages.clone(),
                vec![Stage::Rerank {
                    model,
                    query,
                    fields,
                    topk_multiple,
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

impl From<Query> for topk_rs::query::Query {
    fn from(query: Query) -> Self {
        Self::new(query.stages.into_iter().map(|s| s.into()).collect())
    }
}
