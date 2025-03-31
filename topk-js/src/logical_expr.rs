use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{
  binary_expr::BinaryOperator, document::Value, my_box::MyBox, unary_expr::UnaryOperator,
};

#[napi]
#[derive(Debug, Clone)]
pub enum LogicalExpression {
  Null,
  Field {
    name: String,
  },
  Literal {
    value: Value,
  },
  Unary {
    op: UnaryOperator,
    #[napi(ts_type = "LogicalExpression")]
    expr: MyBox<LogicalExpression>,
  },
  Binary {
    #[napi(ts_type = "LogicalExpression")]
    left: MyBox<LogicalExpression>,
    op: BinaryOperator,
    #[napi(ts_type = "LogicalExpression")]
    right: MyBox<LogicalExpression>,
  },
}

impl LogicalExpression {
  // pub fn eq(&self, other: &LogicalExpression) -> bool {
  //   match (self, other) {
  //     (LogicalExpression::Null, LogicalExpression::Null) => true,
  //     (LogicalExpression::Field { name: l }, LogicalExpression::Field { name: r }) => l == r,
  //     (LogicalExpression::Literal { value: l }, LogicalExpression::Literal { value: r }) => l == r,
  //     (
  //       LogicalExpression::Unary {
  //         op: l,
  //         expr: l_expr,
  //       },
  //       LogicalExpression::Unary {
  //         op: r,
  //         expr: r_expr,
  //       },
  //     ) => l == r && l_expr.as_ref() == r_expr.as_ref(),
  //     (
  //       LogicalExpression::Binary {
  //         left: l,
  //         op: l_op,
  //         right: l_right,
  //       },
  //       LogicalExpression::Binary {
  //         left: r,
  //         op: r_op,
  //         right: r_right,
  //       },
  //     ) => l.get() == r.get() && l_op == r_op && l_right.get() == r_right.get(),
  //     _ => false,
  //   }
  // }

  // pub fn to_string(&self) -> String {
  //   match self {
  //     Self::Null => "Null".to_string(),
  //     Self::Field { name } => format!("field({})", name),
  //     Self::Literal { value } => format!("literal({:?})", value),
  //     Self::Unary { op, expr } => {
  //       format!("Unary(op={:?}, expr={})", op, expr.get().to_string())
  //     }
  //     Self::Binary { left, op, right } => {
  //       format!(
  //         "Binary(left={}, op={:?}, right={})",
  //         left.as_ref().get().to_string(),
  //         op,
  //         right.get().to_string()
  //       )
  //     }
  //   }
  // }

  pub fn equals(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::Eq,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn not_equals(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::NotEq,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn less_than(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::Lt,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn less_than_or_equal(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::LtEq,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn greater_than(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::Gt,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn greater_than_or_equal(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::GtEq,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn add(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::Add,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn subtract(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::Sub,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn multiply(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::Mul,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn divide(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::Div,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn and(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::And,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn or(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::Or,
      right: MyBox(Box::new(other.clone())),
    }
  }

  pub fn starts_with(&self, other: &LogicalExpression) -> LogicalExpression {
    LogicalExpression::Binary {
      left: MyBox(Box::new(self.clone())),
      op: BinaryOperator::StartsWith,
      right: MyBox(Box::new(other.clone())),
    }
  }
}
