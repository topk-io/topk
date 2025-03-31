use crate::{
  document::Value, filter_expr::FilterExpression, function_expr::FunctionExpression,
  logical_expr::LogicalExpression, select_expr::SelectExpression, text_expr::TextExpression,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
#[derive(Debug, Clone)]
pub enum Expression {
  SelectExpression { expr: SelectExpression },
  FilterExpression { expr: FilterExpression },
  LogicalExpression { expr: LogicalExpression },
  TextExpression { expr: TextExpression },
}

#[napi]
#[derive(Debug, Clone)]
pub struct Expr {
  expr: Expression,
}

#[napi]
impl Expr {
  #[napi(factory)]
  pub fn create_match(field: String) -> Expr {
    Expr {
      expr: Expression::TextExpression {
        expr: TextExpression::Terms {
          all: true,
          terms: vec![crate::text_expr::Term {
            token: field,
            field: None,
            weight: 1.0,
          }],
        },
      },
    }
  }

  #[napi(factory)]
  pub fn create_field(name: String) -> Expr {
    Expr {
      expr: Expression::LogicalExpression {
        expr: LogicalExpression::Field { name },
      },
    }
  }

  #[napi(factory)]
  pub fn create_function(expr: FunctionExpression) -> Expr {
    Expr {
      expr: Expression::SelectExpression {
        expr: SelectExpression::Function { expr },
      },
    }
  }

  #[napi(getter)]
  pub fn get_expr(&self) -> Expression {
    self.expr.clone()
  }

  #[napi]
  pub fn eq(&self, value: Value) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.equals(&LogicalExpression::Literal { value }),
        },
      },
      _ => unreachable!("eq can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn not_eq(&self, value: Value) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.not_equals(&LogicalExpression::Literal { value }),
        },
      },
      _ => unreachable!("not_eq can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn lt(&self, value: f64) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.less_than(&LogicalExpression::Literal {
            value: Value::F64(value),
          }),
        },
      },
      _ => unreachable!("lt can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn lte(&self, value: f64) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.less_than_or_equal(&LogicalExpression::Literal {
            value: Value::F64(value),
          }),
        },
      },
      _ => unreachable!("lte can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn gt(&self, value: f64) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.greater_than(&LogicalExpression::Literal {
            value: Value::F64(value),
          }),
        },
      },
      _ => unreachable!("gt can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn gte(&self, value: f64) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.greater_than_or_equal(&LogicalExpression::Literal {
            value: Value::F64(value),
          }),
        },
      },
      _ => unreachable!("gte can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn add(&self, value: f64) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.add(&LogicalExpression::Literal {
            value: Value::F64(value),
          }),
        },
      },
      _ => unreachable!("add can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn subtract(&self, value: f64) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.subtract(&LogicalExpression::Literal {
            value: Value::F64(value),
          }),
        },
      },
      _ => unreachable!("subtract can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn multiply(&self, value: f64) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.multiply(&LogicalExpression::Literal {
            value: Value::F64(value),
          }),
        },
      },
      _ => unreachable!("multiply can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn divide(&self, value: f64) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.divide(&LogicalExpression::Literal {
            value: Value::F64(value),
          }),
        },
      },
      _ => unreachable!("divide can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn and(&self, other: Expr) -> Expr {
    match &self.expr {
      Expression::TextExpression { expr } => {
        if let Expression::TextExpression { expr: other_expr } = &other.expr {
          Self {
            expr: Expression::TextExpression {
              expr: expr.and(other_expr),
            },
          }
        } else {
          unreachable!("and can only be called on text expressions")
        }
      }
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.and(&other.expr.into()),
        },
      },
      _ => unreachable!("and can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn or(&self, other: Expr) -> Expr {
    match &self.expr {
      Expression::TextExpression { expr } => {
        if let Expression::TextExpression { expr: other_expr } = &other.expr {
          Self {
            expr: Expression::TextExpression {
              expr: expr.or(other_expr),
            },
          }
        } else {
          unreachable!("or can only be called on text expressions")
        }
      }
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.or(&other.expr.into()),
        },
      },
      _ => unreachable!("or can only be called on logical expressions"),
    }
  }

  #[napi]
  pub fn starts_with(&self, value: String) -> Expr {
    match &self.expr {
      Expression::LogicalExpression { expr } => Self {
        expr: Expression::LogicalExpression {
          expr: expr.starts_with(&LogicalExpression::Literal {
            value: Value::String(value),
          }),
        },
      },
      _ => unreachable!("starts_with can only be called on logical expressions"),
    }
  }
}

impl FromNapiValue for Expr {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
  ) -> Result<Self, napi::Status> {
    let object = Object::from_napi_value(env, value)?;
    let expr: Expression = object.get("expr")?.ok_or_else(|| {
      napi::Error::new(
        napi::Status::GenericFailure,
        "Field object missing 'expr' property".to_string(),
      )
    })?;

    Ok(Self { expr })
  }
}

#[napi]
pub fn field(name: String) -> Expr {
  Expr::create_field(name)
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct MatchOptions {
  pub field: String,
  pub weight: f64,
}

#[napi(js_name = "match")]
pub fn match_(token: String, options: Option<MatchOptions>) -> Expr {
  // let term = crate::text_expr::Term {
  //   token,
  //   field: options.as_ref().map(|o| o.field.clone()),
  //   weight: options.as_ref().map_or(1.0, |o| o.weight),
  // };

  // let text_expr = TextExpression::Terms {
  //   all: options.is_none(), // If options is not passed, all will be true
  //   terms: vec![term],
  // };

  Expr::create_match(token)
}

impl From<Expression> for LogicalExpression {
  fn from(expr: Expression) -> Self {
    match expr {
      Expression::LogicalExpression { expr } => expr.into(),
      _ => unreachable!("LogicalExpression can only be created from LogicalExpression"),
    }
  }
}

impl From<Expression> for FilterExpression {
  fn from(expr: Expression) -> Self {
    match expr {
      Expression::FilterExpression { expr } => expr,
      Expression::TextExpression { expr } => FilterExpression::Text { expr },
      _ => {
        println!("expr: {:?}", expr);
        unreachable!(
          "FilterExpression can only be created from TextExpression or LogicalExpression"
        )
      }
    }
  }
}

impl From<Expr> for SelectExpression {
  fn from(expr: Expr) -> Self {
    match expr.expr {
      Expression::LogicalExpression { expr } => SelectExpression::Logical { expr },
      Expression::SelectExpression { expr } => expr,
      Expression::FilterExpression { .. } => {
        unreachable!("FilterExpression cannot be converted to SelectExpression")
      }
      Expression::TextExpression { .. } => {
        unreachable!("TextExpression cannot be converted to SelectExpression")
      }
    }
  }
}
