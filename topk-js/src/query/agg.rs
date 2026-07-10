use crate::expr::aggregate::{AggregateExpression, AggregateExpressionUnion};
use napi_derive::napi;

/// Count the number of non-null values for the given field.
/// If not provided, count the number of rows in the input.
#[napi(namespace = "query_agg", ts_return_type = "query.AggregateExpression")]
pub fn count(field: Option<String>) -> AggregateExpression {
    AggregateExpression(AggregateExpressionUnion::Count { field })
}

/// Sum the values of the given field.
#[napi(namespace = "query_agg", ts_return_type = "query.AggregateExpression")]
pub fn sum(field: String) -> AggregateExpression {
    AggregateExpression(AggregateExpressionUnion::Sum { field })
}

/// Find the minimum value of the given field.
#[napi(namespace = "query_agg", ts_return_type = "query.AggregateExpression")]
pub fn min(field: String) -> AggregateExpression {
    AggregateExpression(AggregateExpressionUnion::Min { field })
}

/// Find the maximum value of the given field.
#[napi(namespace = "query_agg", ts_return_type = "query.AggregateExpression")]
pub fn max(field: String) -> AggregateExpression {
    AggregateExpression(AggregateExpressionUnion::Max { field })
}

/// Calculate the average value of the given field.
#[napi(namespace = "query_agg", ts_return_type = "query.AggregateExpression")]
pub fn avg(field: String) -> AggregateExpression {
    AggregateExpression(AggregateExpressionUnion::Avg { field })
}
