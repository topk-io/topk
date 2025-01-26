use super::{
    filter_expr::FilterExpressionUnion,
    logical_expr::LogicalExpression,
    select_expr::{SelectExpression, SelectExpressionUnion},
    stage::Stage,
};
use pyo3::prelude::*;
use std::collections::HashMap;

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
}

impl From<Query> for topk_protos::v1::data::Query {
    fn from(query: Query) -> Self {
        Self {
            stages: query.stages.into_iter().map(|s| s.into()).collect(),
        }
    }
}
