use chrono::{TimeZone, Utc};
use topk_rs::proto::v1::data::LogicalExpr;
use topk_rs::query::field;

use super::{Error, Zone};
use crate::api::DateHistogramBody;

// A date_histogram bucket width. Fixed widths are an exact number of millis and bucket by integer
// division. Months vary in length, so month/quarter/year bucket on a month count instead — they
// differ only in how many months a bucket spans. Calendar day and below are exact in UTC, so they
// are Fixed too.
pub enum Interval {
    Fixed(i64),
    Months(i64),
}

// Buckets are computed on `ts + offset` and shifted back when reported, so a whole histogram is
// aligned to one constant offset.
pub fn offset_millis(h: &DateHistogramBody) -> Result<i64, Error> {
    let Some(tz) = h.time_zone.as_deref() else {
        return Ok(0);
    };

    match Zone::parse(tz)? {
        Zone::Fixed(offset) => Ok(offset.local_minus_utc() as i64 * 1_000),
        Zone::Named(_) => Err(Error::BadRequest(format!(
            "date_histogram time_zone [{tz}] must be a numeric offset like +02:00; \
             named zones observe DST, which bucketing cannot follow"
        ))),
    }
}

pub fn interval(h: &DateHistogramBody) -> Result<Interval, Error> {
    match (&h.fixed_interval, &h.calendar_interval) {
        (Some(_), Some(_)) => Err(Error::BadRequest(
            "date_histogram accepts either fixed_interval or calendar_interval, not both".into(),
        )),
        (Some(fixed), None) => fixed_interval(fixed).map(Interval::Fixed),
        (None, Some(calendar)) => calendar_interval(calendar),
        (None, None) => Err(Error::BadRequest(
            "date_histogram requires fixed_interval or calendar_interval".into(),
        )),
    }
}

// `<n><unit>` where unit is ms/s/m/h/d — ES rejects calendar units (w/M/y) here because their
// length varies.
fn fixed_interval(value: &str) -> Result<i64, Error> {
    let split = value
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| Error::BadRequest(format!("invalid fixed_interval [{value}]")))?;
    let (n, unit) = value.split_at(split);
    let n: i64 = n
        .parse()
        .map_err(|_| Error::BadRequest(format!("invalid fixed_interval [{value}]")))?;

    let millis = match unit {
        "ms" => 1,
        "s" => 1_000,
        "m" => 60 * 1_000,
        "h" => 60 * 60 * 1_000,
        "d" => 24 * 60 * 60 * 1_000,
        _ => return Err(Error::BadRequest(format!("invalid fixed_interval [{value}]"))),
    };

    match n.checked_mul(millis).filter(|m| *m > 0) {
        Some(millis) => Ok(millis),
        None => Err(Error::BadRequest(format!("invalid fixed_interval [{value}]"))),
    }
}

// ES spells these either as a word (`month`) or as `1`-prefixed shorthand (`1M`); multiples other
// than 1 are rejected, as in ES.
fn calendar_interval(value: &str) -> Result<Interval, Error> {
    const DAY: i64 = 24 * 60 * 60 * 1_000;

    match value {
        "second" | "1s" => Ok(Interval::Fixed(1_000)),
        "minute" | "1m" => Ok(Interval::Fixed(60 * 1_000)),
        "hour" | "1h" => Ok(Interval::Fixed(60 * 60 * 1_000)),
        "day" | "1d" => Ok(Interval::Fixed(DAY)),
        "week" | "1w" => Ok(Interval::Fixed(7 * DAY)),
        "month" | "1M" => Ok(Interval::Months(1)),
        "quarter" | "1q" => Ok(Interval::Months(3)),
        "year" | "1y" => Ok(Interval::Months(12)),
        _ => Err(Error::BadRequest(format!(
            "invalid calendar_interval [{value}]"
        ))),
    }
}

// The grouping expression whose value identifies a document's bucket. `offset` shifts the field
// so buckets align to a zone other than UTC; `bucket_start` shifts the reported key back.
pub fn bucket_key(name: &str, interval: &Interval, offset: i64) -> LogicalExpr {
    let at = || match offset {
        0 => field(name),
        offset => field(name).add(LogicalExpr::literal(offset)),
    };

    match interval {
        Interval::Fixed(millis) => at().div(LogicalExpr::literal(*millis)),
        // Months elapsed since year 0, divided by the bucket's width in months. Folding the year
        // in is what keeps January 2025 and January 2026 in different buckets.
        Interval::Months(n) => at()
            .date_part("year")
            .mul(LogicalExpr::literal(12))
            .add(at().date_part("month"))
            .sub(LogicalExpr::literal(1))
            .div(LogicalExpr::literal(*n)),
    }
}

// Inverse of `bucket_key`: the timestamp ES reports for bucket `index`.
pub fn bucket_start(interval: &Interval, index: i64) -> Option<i64> {
    match interval {
        Interval::Fixed(millis) => index.checked_mul(*millis),
        Interval::Months(n) => {
            let months = index.checked_mul(*n)?;
            Utc.with_ymd_and_hms(
                months.div_euclid(12) as i32,
                (months.rem_euclid(12) + 1) as u32,
                1,
                0,
                0,
                0,
            )
            .single()
            .map(|d| d.timestamp_millis())
        }
    }
}
