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
    pub fn sum(field: impl Into<String>) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Sum(aggregate_expr::Sum {
                field: field.into(),
            })),
        }
    }

    /// Find the minimum value of the given field.
    pub fn min(field: impl Into<String>) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Min(aggregate_expr::Min {
                field: field.into(),
            })),
        }
    }

    /// Find the maximum value of the given field.
    pub fn max(field: impl Into<String>) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Max(aggregate_expr::Max {
                field: field.into(),
            })),
        }
    }

    /// Calculate the average value of the given field.
    pub fn avg(field: impl Into<String>) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Avg(aggregate_expr::Average {
                field: field.into(),
            })),
        }
    }

    /// Calculate an approximate quantile of the given numeric field.
    /// `q` must be finite and in the inclusive range `[0, 1]`.
    pub fn quantile(field: impl Into<String>, q: f64) -> Self {
        Self {
            op: Some(aggregate_expr::Op::Quantile(aggregate_expr::Quantile {
                field: field.into(),
                q,
            })),
        }
    }

    /// Count the approximate number of distinct non-null values of the given field.
    pub fn count_distinct(field: impl Into<String>) -> Self {
        Self {
            op: Some(aggregate_expr::Op::CountDistinct(
                aggregate_expr::CountDistinct {
                    field: field.into(),
                },
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantile_constructs_proto() {
        let expr = AggregateExpr::quantile("score", 0.99);

        assert_eq!(
            expr.op,
            Some(aggregate_expr::Op::Quantile(aggregate_expr::Quantile {
                field: "score".to_string(),
                q: 0.99,
            }))
        );
    }

    #[test]
    fn count_distinct_constructs_proto() {
        let expr = AggregateExpr::count_distinct("author");

        assert_eq!(
            expr.op,
            Some(aggregate_expr::Op::CountDistinct(
                aggregate_expr::CountDistinct {
                    field: "author".to_string(),
                }
            ))
        );
    }
}
