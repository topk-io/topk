use chrono::{DateTime, Datelike, NaiveDate, Utc};
use topk_rs::proto::v1::data::LogicalExpr;
use topk_rs::query::field;

use super::Error;

const HOUR: i64 = 60 * 60 * 1_000;
const DAY: i64 = 24 * HOUR;

pub struct Bucketing {
    kind: Kind,

    shift: i64,
}

enum Kind {
    Fixed { width: i64, offset: i64 },
    Months { count: i64, offset: i64 },
}

#[derive(Clone, Copy)]
pub enum Interval {
    Fixed(i64),
    Week,
    Months(i64),
}

pub fn bucketing(interval: Interval, offset: i64, shift: i64) -> Bucketing {
    let kind = match interval {
        Interval::Fixed(width) => Kind::Fixed { width, offset },
        Interval::Week => Kind::Fixed {
            width: 7 * DAY,
            offset: offset + 3 * DAY,
        },
        Interval::Months(count) => Kind::Months { count, offset },
    };

    Bucketing { kind, shift }
}

// `+6h` / `-1d`: a signed fixed duration added to every bucket boundary.
pub fn parse_offset(offset: &str) -> Result<i64, Error> {
    let (sign, duration) = match offset.strip_prefix('-') {
        Some(d) => (-1, d),
        None => (1, offset.strip_prefix('+').unwrap_or(offset)),
    };
    let millis = parse_fixed_interval(duration)
        .map_err(|_| Error::BadRequest(format!("invalid offset [{offset}]")))?;
    Ok(sign * millis)
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

pub fn parse_calendar_interval(value: &str) -> Result<Interval, Error> {
    match value {
        "second" | "1s" => Ok(Interval::Fixed(1_000)),
        "minute" | "1m" => Ok(Interval::Fixed(60 * 1_000)),
        "hour" | "1h" => Ok(Interval::Fixed(HOUR)),
        "day" | "1d" => Ok(Interval::Fixed(DAY)),
        "week" | "1w" => Ok(Interval::Week),
        "month" | "1M" => Ok(Interval::Months(1)),
        "quarter" | "1q" => Ok(Interval::Months(3)),
        "year" | "1y" => Ok(Interval::Months(12)),
        _ => Err(Error::BadRequest(format!(
            "invalid calendar_interval [{value}]"
        ))),
    }
}

impl Bucketing {
    fn shifted(&self, name: &str, offset: i64) -> LogicalExpr {
        match offset - self.shift {
            0 => field(name),
            offset => field(name).add(LogicalExpr::literal(offset)),
        }
    }

    pub fn key_expr(&self, name: &str) -> LogicalExpr {
        match &self.kind {
            Kind::Fixed { width, offset } => self
                .shifted(name, *offset)
                .div(LogicalExpr::literal(*width)),
            Kind::Months { count, offset } => self
                .shifted(name, *offset)
                .date_part("year")
                .mul(LogicalExpr::literal(12))
                .add(self.shifted(name, *offset).date_part("month"))
                .sub(LogicalExpr::literal(1))
                .div(LogicalExpr::literal(*count)),
        }
    }

    pub fn start_of_index(&self, index: i64) -> Option<i64> {
        let start = match &self.kind {
            Kind::Fixed { width, offset } => index.checked_mul(*width)?.checked_sub(*offset)?,
            Kind::Months { count, offset } => {
                month_start(index.checked_mul(*count)?)?.checked_sub(*offset)?
            }
        };
        start.checked_add(self.shift)
    }

    pub fn floor_unshifted(&self, t: i64) -> Option<i64> {
        Some(match &self.kind {
            Kind::Fixed { width, offset } => {
                t.checked_add(*offset)?.div_euclid(*width) * *width - *offset
            }
            Kind::Months { count, offset } => {
                let at = DateTime::<Utc>::from_timestamp_millis(t.checked_add(*offset)?)?;
                let months =
                    (at.year() as i64 * 12 + at.month0() as i64).div_euclid(*count) * *count;
                month_start(months)?.checked_sub(*offset)?
            }
        })
    }

    pub fn extended_bound(&self, t: i64) -> Option<i64> {
        self.floor_unshifted(t)?.checked_add(self.shift)
    }

    // Start of the bucket after the one starting at `start`; drives empty-bucket filling.
    pub fn next(&self, start: i64) -> Option<i64> {
        let start = start.checked_sub(self.shift)?;
        let next = match &self.kind {
            Kind::Fixed { width, .. } => start.checked_add(*width)?,
            Kind::Months { count, offset } => {
                let at = DateTime::<Utc>::from_timestamp_millis(start.checked_add(*offset)?)?;
                let months = at.year() as i64 * 12 + at.month0() as i64 + count;
                month_start(months)?.checked_sub(*offset)?
            }
        };
        next.checked_add(self.shift)
    }
}

fn month_start(months: i64) -> Option<i64> {
    Some(
        NaiveDate::from_ymd_opt(
            months.div_euclid(12) as i32,
            (months.rem_euclid(12) + 1) as u32,
            1,
        )?
        .and_hms_opt(0, 0, 0)?
        .and_utc()
        .timestamp_millis(),
    )
}
