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

impl Into<topk_rs::expr::text::TextExpr> for TextExpr {
    fn into(self) -> topk_rs::expr::text::TextExpr {
        match self {
            TextExpr::Terms { all, terms } => topk_rs::expr::text::TextExpr::Terms {
                all,
                terms: terms.into_iter().map(|t| t.into()).collect(),
            },
            TextExpr::And { left, right } => topk_rs::expr::text::TextExpr::And {
                left: Box::new(left.get().clone().into()),
                right: Box::new(right.get().clone().into()),
            },
            TextExpr::Or { left, right } => topk_rs::expr::text::TextExpr::Or {
                left: Box::new(left.get().clone().into()),
                right: Box::new(right.get().clone().into()),
            },
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

impl Into<topk_rs::expr::text::Term> for Term {
    fn into(self) -> topk_rs::expr::text::Term {
        topk_rs::expr::text::Term {
            token: self.token,
            field: self.field,
            weight: self.weight,
        }
    }
}
