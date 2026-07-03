use crate::proto::data::v1::{aggregate_expr, AggregateExpr};

impl AggregateExpr {
    /// Count the number of non-null values for the given field.
    /// If not provided, count the number of rows in the input.
    pub fn count(field: Option<String>) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Count(aggregate_expr::Count {
                field: field.map(|f| f.into()),
            })),
        }
    }

    /// Sum the values of the given field.
    pub fn sum(field: String) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Sum(aggregate_expr::Sum {
                field: field.into(),
            })),
        }
    }

    /// Find the minimum value of the given field.
    pub fn min(field: String) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Min(aggregate_expr::Min {
                field: field.into(),
            })),
        }
    }

    /// Find the maximum value of the given field.
    pub fn max(field: String) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Max(aggregate_expr::Max {
                field: field.into(),
            })),
        }
    }

    /// Calculate the average value of the given field.
    pub fn avg(field: String) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Avg(aggregate_expr::Average {
                field: field.into(),
            })),
        }
    }
}
