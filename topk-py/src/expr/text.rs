use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum TextExpr {
    Terms {
        all: bool,
        terms: Vec<Term>,
    },
    And {
        left: Py<TextExpr>,
        right: Py<TextExpr>,
    },
    Or {
        left: Py<TextExpr>,
        right: Py<TextExpr>,
    },
}

#[pymethods]
impl TextExpr {
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }

    fn __and__(&self, py: Python<'_>, other: TextExpr) -> PyResult<Self> {
        Ok(Self::And {
            left: Py::new(py, self.clone())?,
            right: Py::new(py, other)?,
        })
    }

    fn __rand__(&self, py: Python<'_>, other: TextExpr) -> PyResult<Self> {
        Ok(Self::And {
            left: Py::new(py, other)?,
            right: Py::new(py, self.clone())?,
        })
    }

    fn __or__(&self, py: Python<'_>, other: TextExpr) -> PyResult<Self> {
        Ok(Self::Or {
            left: Py::new(py, self.clone())?,
            right: Py::new(py, other)?,
        })
    }

    fn __ror__(&self, py: Python<'_>, other: TextExpr) -> PyResult<Self> {
        Ok(Self::Or {
            left: Py::new(py, other)?,
            right: Py::new(py, self.clone())?,
        })
    }
}

impl From<TextExpr> for topk_rs::proto::v1::data::TextExpr {
    fn from(expr: TextExpr) -> Self {
        match expr {
            TextExpr::Terms { all, terms } => topk_rs::proto::v1::data::TextExpr::terms(
                all,
                terms.into_iter().map(|term| term.into()).collect(),
            ),
            TextExpr::And { left, right } => {
                let left: topk_rs::proto::v1::data::TextExpr = left.get().clone().into();
                let right: topk_rs::proto::v1::data::TextExpr = right.get().clone().into();
                left.and(right)
            }
            TextExpr::Or { left, right } => {
                let left: topk_rs::proto::v1::data::TextExpr = left.get().clone().into();
                let right: topk_rs::proto::v1::data::TextExpr = right.get().clone().into();
                left.or(right)
            }
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub token: String,
    pub field: Option<String>,
    pub weight: f32,
}

impl From<Term> for topk_rs::proto::v1::data::text_expr::Term {
    fn from(term: Term) -> Self {
        topk_rs::proto::v1::data::text_expr::Term {
            token: term.token,
            field: term.field,
            weight: term.weight,
        }
    }
}
