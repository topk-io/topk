use chrono::DateTime;
use topk_rs::proto::v1::data::LogicalExpr;
use topk_rs::query::field;

use super::{Error, Unit, Zone};

const HOUR: i64 = 60 * 60 * 1_000;
const DAY: i64 = 24 * HOUR;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    Calendar(Unit),
    Fixed(i64),
}

pub fn parse_fixed_interval(value: &str) -> Result<i64, Error> {
    let invalid = || Error::BadRequest(format!("invalid fixed_interval [{value}]"));

    let split = value
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(invalid)?;
    let (n, unit) = value.split_at(split);
    let n: i64 = n.parse().map_err(|_| invalid())?;

    let millis = match unit {
        "ms" => 1,
        "s" => 1_000,
        "m" => 60 * 1_000,
        "h" => HOUR,
        "d" => DAY,
        _ => return Err(invalid()),
    };

    n.checked_mul(millis).filter(|m| *m > 0).ok_or_else(invalid)
}

pub fn parse_calendar_interval(value: &str) -> Result<Unit, Error> {
    match value {
        "second" | "1s" => Ok(Unit::Second),
        "minute" | "1m" => Ok(Unit::Minute),
        "hour" | "1h" => Ok(Unit::Hour),
        "day" | "1d" => Ok(Unit::Day),
        "week" | "1w" => Ok(Unit::Week),
        "month" | "1M" => Ok(Unit::Month),
        "quarter" | "1q" => Ok(Unit::Quarter),
        "year" | "1y" => Ok(Unit::Year),
        _ => Err(Error::BadRequest(format!(
            "invalid calendar_interval [{value}]"
        ))),
    }
}

impl Interval {
    pub fn key_expr(&self, name: &str, zone: &Zone) -> LogicalExpr {
        match self {
            Interval::Calendar(unit) => field(name).date_trunc(unit.as_str(), zone.to_string()),
            Interval::Fixed(width) => field(name)
                .div(LogicalExpr::literal(*width))
                .mul(LogicalExpr::literal(*width)),
        }
    }

    pub fn next(&self, start: i64, zone: Option<&Zone>) -> Option<i64> {
        match self {
            Interval::Fixed(width) => start.checked_add(*width),
            Interval::Calendar(unit) => {
                let zone = zone.copied().unwrap_or(Zone::UTC);
                let local = zone.local(DateTime::from_timestamp_millis(start)?);
                zone.utc(super::add(local, *unit, 1).ok()?)
                    .map(|at| at.timestamp_millis())
            }
        }
    }
}
