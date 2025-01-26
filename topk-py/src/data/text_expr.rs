use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum TextExpression {
    Terms {
        all: bool,
        terms: Vec<Term>,
    },
    And {
        left: Py<TextExpression>,
        right: Py<TextExpression>,
    },
    Or {
        left: Py<TextExpression>,
        right: Py<TextExpression>,
    },
}

#[pymethods]
impl TextExpression {
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }
    fn __and__(&self, py: Python<'_>, other: TextExpression) -> PyResult<Self> {
        Ok(Self::And {
            left: Py::new(py, self.clone())?,
            right: Py::new(py, other)?,
        })
    }

    fn __rand__(&self, py: Python<'_>, other: TextExpression) -> PyResult<Self> {
        Ok(Self::And {
            left: Py::new(py, other)?,
            right: Py::new(py, self.clone())?,
        })
    }

    fn __or__(&self, py: Python<'_>, other: TextExpression) -> PyResult<Self> {
        Ok(Self::Or {
            left: Py::new(py, self.clone())?,
            right: Py::new(py, other)?,
        })
    }

    fn __ror__(&self, py: Python<'_>, other: TextExpression) -> PyResult<Self> {
        Ok(Self::Or {
            left: Py::new(py, other)?,
            right: Py::new(py, self.clone())?,
        })
    }
}

impl Into<topk_protos::v1::data::TextExpr> for TextExpression {
    fn into(self) -> topk_protos::v1::data::TextExpr {
        match self {
            TextExpression::Terms { all, terms } => topk_protos::v1::data::TextExpr::terms(
                all,
                terms.into_iter().map(|t| t.into()).collect(),
            ),
            TextExpression::And { left, right } => topk_protos::v1::data::TextExpr::and(
                left.get().clone().into(),
                right.get().clone().into(),
            ),
            TextExpression::Or { left, right } => topk_protos::v1::data::TextExpr::or(
                left.get().clone().into(),
                right.get().clone().into(),
            ),
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

impl Into<topk_protos::v1::data::text_expr::Term> for Term {
    fn into(self) -> topk_protos::v1::data::text_expr::Term {
        topk_protos::v1::data::text_expr::Term {
            token: self.token,
            field: self.field,
            weight: self.weight,
        }
    }
}
