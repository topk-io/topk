use crate::expr::aggregate::AggregateExpr;
use crate::expr::filter::FilterExprUnion;
use crate::expr::logical::LogicalExpr;
use crate::expr::select::{SelectExpr, SelectExprUnion};
use pyo3::{exceptions::PyTypeError, prelude::*, types::PyString};
use std::collections::HashMap;

use super::stage::Stage;

#[pyclass]
pub struct ConsistencyLevel(String);

#[pymethods]
impl ConsistencyLevel {
    #[classattr]
    #[pyo3(name = "Indexed")]
    fn indexed() -> Self {
        ConsistencyLevel("indexed".to_string())
    }

    #[classattr]
    #[pyo3(name = "Strong")]
    fn strong() -> Self {
        ConsistencyLevel("strong".to_string())
    }

    fn __repr__(&self) -> String {
        let variant = match self.0.as_str() {
            "indexed" => "Indexed",
            "strong" => "Strong",
            _ => &self.0,
        };
        format!("ConsistencyLevel.{variant}")
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for ConsistencyLevel {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(cl) = obj.extract::<PyRef<ConsistencyLevel>>() {
            return Ok(ConsistencyLevel(cl.0.clone()));
        }
        match obj.cast_exact::<PyString>() {
            Ok(val) => match val.extract::<&str>()? {
                s @ ("indexed" | "strong") => Ok(ConsistencyLevel(s.to_string())),
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

impl From<ConsistencyLevel> for topk_rs::proto::v1::data::ConsistencyLevel {
    fn from(c: ConsistencyLevel) -> Self {
        match c.0.as_str() {
            "indexed" => topk_rs::proto::v1::data::ConsistencyLevel::Indexed,
            "strong" => topk_rs::proto::v1::data::ConsistencyLevel::Strong,
            _ => unreachable!("invalid ConsistencyLevel value: {}", c.0),
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

    #[deprecated(note = "Use .sort(expr, asc).limit(k) instead")]
    #[pyo3(signature = (expr, k, asc=false))]
    pub fn topk(&self, expr: LogicalExpr, k: u64, asc: bool) -> PyResult<Self> {
        Ok(Self {
            stages: [
                self.stages.clone(),
                vec![
                    Stage::Sort {
                        expr: expr.into(),
                        asc,
                    },
                    Stage::Limit { k },
                ],
            ]
            .concat(),
        })
    }

    #[pyo3(signature = (k))]
    pub fn limit(&self, k: u64) -> PyResult<Self> {
        Ok(Self {
            stages: [self.stages.clone(), vec![Stage::Limit { k }]].concat(),
        })
    }

    #[pyo3(signature = (offset))]
    pub fn offset(&self, offset: u64) -> PyResult<Self> {
        Ok(Self {
            stages: [self.stages.clone(), vec![Stage::Offset { offset }]].concat(),
        })
    }

    #[pyo3(signature = (expr, asc=true))]
    pub fn sort(&self, expr: LogicalExpr, asc: bool) -> PyResult<Self> {
        Ok(Self {
            stages: [
                self.stages.clone(),
                vec![Stage::Sort {
                    expr: expr.into(),
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

    pub fn group_by(
        &self,
        keys: HashMap<String, LogicalExpr>,
        aggs: HashMap<String, AggregateExpr>,
    ) -> PyResult<Self> {
        Ok(Self {
            stages: [self.stages.clone(), vec![Stage::GroupBy { keys, aggs }]].concat(),
        })
    }
}

impl From<Query> for topk_rs::proto::v1::data::Query {
    fn from(query: Query) -> Self {
        topk_rs::proto::v1::data::Query::new(query.stages.into_iter().map(|s| s.into()).collect())
    }
}
