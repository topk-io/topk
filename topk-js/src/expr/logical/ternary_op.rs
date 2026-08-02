use napi_derive::napi;

#[napi(string_enum = "snake_case", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DatePart {
    Year,
    Month,
    Week,
    Day,
    DayOfYear,
    DayOfWeek,
    Hour,
    Minute,
    Second,
    Millisecond,
}

impl std::fmt::Display for DatePart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DatePart::Year => "year",
            DatePart::Month => "month",
            DatePart::Week => "week",
            DatePart::Day => "day",
            DatePart::DayOfYear => "day_of_year",
            DatePart::DayOfWeek => "day_of_week",
            DatePart::Hour => "hour",
            DatePart::Minute => "minute",
            DatePart::Second => "second",
            DatePart::Millisecond => "millisecond",
        })
    }
}

#[napi(string_enum = "snake_case", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interval {
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Interval::Millisecond => "millisecond",
            Interval::Second => "second",
            Interval::Minute => "minute",
            Interval::Hour => "hour",
            Interval::Day => "day",
            Interval::Week => "week",
        })
    }
}

/// @ignore
#[napi(string_enum = "camelCase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TernaryOperator {
    Choose,
    RegexpMatch,
    Elapsed,
}

impl Into<topk_rs::proto::v1::data::logical_expr::ternary_op::Op> for TernaryOperator {
    fn into(self) -> topk_rs::proto::v1::data::logical_expr::ternary_op::Op {
        match self {
            TernaryOperator::Choose => {
                topk_rs::proto::v1::data::logical_expr::ternary_op::Op::Choose
            }
            TernaryOperator::RegexpMatch => {
                topk_rs::proto::v1::data::logical_expr::ternary_op::Op::RegexpMatch
            }
            TernaryOperator::Elapsed => {
                topk_rs::proto::v1::data::logical_expr::ternary_op::Op::Elapsed
            }
        }
    }
}
