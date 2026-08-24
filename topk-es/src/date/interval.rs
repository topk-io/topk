use chrono::{TimeZone, Utc};

use super::Error;
use crate::api::DateHistogramBody;

// A date_histogram bucket width. Fixed widths bucket by integer division; calendar years and
// months are irregular, so they bucket by date part instead. Calendar day/hour/minute/second and
// week are exact in UTC, so they collapse to Fixed.
pub enum Interval {
    Fixed(i64),
    Year,
    Month,
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
        "month" | "1M" => Ok(Interval::Month),
        "year" | "1y" => Ok(Interval::Year),
        _ => Err(Error::BadRequest(format!(
            "invalid calendar_interval [{value}]"
        ))),
    }
}

// Bucket index -> the timestamp ES reports as the bucket key.
pub fn bucket_start(interval: &Interval, index: i64) -> Option<i64> {
    match interval {
        Interval::Fixed(millis) => index.checked_mul(*millis),
        Interval::Year => Utc
            .with_ymd_and_hms(index as i32, 1, 1, 0, 0, 0)
            .single()
            .map(|d| d.timestamp_millis()),
        Interval::Month => Utc
            .with_ymd_and_hms(
                index.div_euclid(12) as i32,
                (index.rem_euclid(12) + 1) as u32,
                1,
                0,
                0,
                0,
            )
            .single()
            .map(|d| d.timestamp_millis()),
    }
}
