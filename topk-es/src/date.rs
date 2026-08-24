use chrono::{DateTime, Datelike, Duration, SecondsFormat, TimeZone, Timelike, Utc};
use topk_rs::proto::v1::control::{field_type, FieldSpec};
use topk_rs::proto::v1::data::Value;

use crate::api::DateHistogramBody;
use crate::Error;

// ES `date` values arrive as ISO-8601 strings (or already-epoch millis) and must land in TopK's
// timestamp column as i64 millis; reads go the other way.
fn parse_millis(value: &str) -> Result<i64, Error> {
    parse_millis_at(value, Utc::now())
}

// `now` is threaded in rather than read here so date math is testable without the wall clock.
fn parse_millis_at(value: &str, now: DateTime<Utc>) -> Result<i64, Error> {
    if let Ok(millis) = value.parse::<i64>() {
        return Ok(millis);
    }

    if let Some(rest) = value.strip_prefix("now") {
        return date_math(rest, now);
    }

    // ES anchors date math to an explicit date with `||`, e.g. `2026-01-15T00:00:00Z||+1d`.
    if let Some((anchor, rest)) = value.split_once("||") {
        let anchor = DateTime::parse_from_rfc3339(anchor)
            .map_err(|_| Error::BadRequest(format!("cannot parse date [{anchor}]")))?
            .with_timezone(&Utc);
        return date_math(rest, anchor);
    }

    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.timestamp_millis())
        .map_err(|_| Error::BadRequest(format!("cannot parse date [{value}]")))
}

// ES date math: a chain of `+Nunit` / `-Nunit` offsets and `/unit` roundings, e.g. `-1d/d`.
fn date_math(expr: &str, anchor: DateTime<Utc>) -> Result<i64, Error> {
    let mut at = anchor;
    let mut rest = expr;

    while !rest.is_empty() {
        let (op, tail) = rest.split_at(1);
        match op {
            "/" => {
                let (unit, tail) = tail.split_at(1.min(tail.len()));
                at = round_down(at, unit_of(unit, expr)?)?;
                rest = tail;
            }
            "+" | "-" => {
                let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
                let (unit, tail) = tail[digits.len()..].split_at(1.min(tail.len() - digits.len()));
                let n: i64 = digits.parse().unwrap_or(1);
                let n = if op == "-" { -n } else { n };
                at = add(at, unit_of(unit, expr)?, n)?;
                rest = tail;
            }
            _ => return Err(Error::BadRequest(format!("cannot parse date [now{expr}]"))),
        }
    }

    Ok(at.timestamp_millis())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
}

fn unit_of(unit: &str, expr: &str) -> Result<Unit, Error> {
    match unit {
        "y" => Ok(Unit::Year),
        "M" => Ok(Unit::Month),
        "w" => Ok(Unit::Week),
        "d" => Ok(Unit::Day),
        "H" | "h" => Ok(Unit::Hour),
        "m" => Ok(Unit::Minute),
        "s" => Ok(Unit::Second),
        _ => Err(Error::BadRequest(format!("cannot parse date [now{expr}]"))),
    }
}

fn add(at: DateTime<Utc>, unit: Unit, n: i64) -> Result<DateTime<Utc>, Error> {
    let shifted = match unit {
        // Calendar units vary in length, so they shift the date rather than add a fixed duration.
        Unit::Year => at.with_year(at.year() + n as i32),
        Unit::Month => {
            let months = at.year() as i64 * 12 + at.month0() as i64 + n;
            Utc.with_ymd_and_hms(
                (months.div_euclid(12)) as i32,
                (months.rem_euclid(12) + 1) as u32,
                at.day(),
                at.hour(),
                at.minute(),
                at.second(),
            )
            .single()
            // `with_ymd_and_hms` stops at seconds; carry the sub-second part over.
            .and_then(|d| d.with_nanosecond(at.nanosecond()))
        }
        Unit::Week => Some(at + Duration::weeks(n)),
        Unit::Day => Some(at + Duration::days(n)),
        Unit::Hour => Some(at + Duration::hours(n)),
        Unit::Minute => Some(at + Duration::minutes(n)),
        Unit::Second => Some(at + Duration::seconds(n)),
    };

    shifted.ok_or_else(|| Error::BadRequest("date math overflowed".to_string()))
}

// `/unit` rounds down to the start of that unit, as ES does.
fn round_down(at: DateTime<Utc>, unit: Unit) -> Result<DateTime<Utc>, Error> {
    let rounded = match unit {
        Unit::Year => Utc.with_ymd_and_hms(at.year(), 1, 1, 0, 0, 0).single(),
        Unit::Month => Utc.with_ymd_and_hms(at.year(), at.month(), 1, 0, 0, 0).single(),
        Unit::Week => at
            .date_naive()
            .week(chrono::Weekday::Mon)
            .first_day()
            .and_hms_opt(0, 0, 0)
            .map(|d| d.and_utc()),
        Unit::Day => at.date_naive().and_hms_opt(0, 0, 0).map(|d| d.and_utc()),
        Unit::Hour => at.with_minute(0).and_then(|d| d.with_second(0)),
        Unit::Minute => at.with_second(0),
        Unit::Second => Some(at),
    };

    rounded
        .and_then(|d| d.with_nanosecond(0))
        .ok_or_else(|| Error::BadRequest("date rounding overflowed".to_string()))
}

pub fn format_millis(millis: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(millis)
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
}

pub fn is_timestamp(spec: &FieldSpec) -> bool {
    matches!(
        spec.data_type.as_ref().and_then(|t| t.data_type.as_ref()),
        Some(field_type::DataType::Timestamp(_))
    )
}

// Coerce a value destined for `spec` onto the epoch millis a timestamp column stores. ES accepts
// an ISO-8601 string, raw millis, or a list of either (`terms`). Non-timestamp fields pass through
// untouched, so a numeric-looking string on a keyword field is never mistaken for a date.
pub fn to_timestamp(spec: Option<&FieldSpec>, value: Value) -> Result<Value, Error> {
    if !spec.is_some_and(is_timestamp) {
        return Ok(value);
    }

    if let Some(s) = value.as_string() {
        return Ok(Value::timestamp(parse_millis(s)?));
    }

    match value.as_string_list() {
        Some(values) => values
            .iter()
            .map(|v| parse_millis(v))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::list),
        None => Ok(value),
    }
}

// Inverse of `to_timestamp`: render a stored timestamp back as the ISO-8601 string ES returns.
// A value that isn't a usable timestamp reads as null rather than leaking raw millis.
pub fn from_timestamp(value: Value) -> Value {
    match value.as_timestamp().and_then(format_millis) {
        Some(iso) => Value::string(iso),
        None => Value::null(),
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn at(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso).unwrap().with_timezone(&Utc)
    }

    fn eval(expr: &str) -> String {
        let now = at("2026-06-15T10:30:45.123Z");
        format_millis(parse_millis_at(expr, now).expect(expr)).unwrap()
    }

    #[test]
    fn plain_values() {
        assert_eq!(eval("2026-01-15T10:00:00Z"), "2026-01-15T10:00:00.000Z");
        assert_eq!(eval("1768471200000"), "2026-01-15T10:00:00.000Z");
        assert_eq!(eval("now"), "2026-06-15T10:30:45.123Z");
    }

    #[test]
    fn offsets() {
        assert_eq!(eval("now-30s"), "2026-06-15T10:30:15.123Z");
        assert_eq!(eval("now-15m"), "2026-06-15T10:15:45.123Z");
        assert_eq!(eval("now+1h"), "2026-06-15T11:30:45.123Z");
        assert_eq!(eval("now-1d"), "2026-06-14T10:30:45.123Z");
        assert_eq!(eval("now-1w"), "2026-06-08T10:30:45.123Z");
        assert_eq!(eval("now-1M"), "2026-05-15T10:30:45.123Z");
        assert_eq!(eval("now-1y"), "2025-06-15T10:30:45.123Z");
    }

    #[test]
    fn rounding() {
        assert_eq!(eval("now/s"), "2026-06-15T10:30:45.000Z");
        assert_eq!(eval("now/m"), "2026-06-15T10:30:00.000Z");
        assert_eq!(eval("now/h"), "2026-06-15T10:00:00.000Z");
        assert_eq!(eval("now/d"), "2026-06-15T00:00:00.000Z");
        assert_eq!(eval("now/w"), "2026-06-15T00:00:00.000Z");
        assert_eq!(eval("now/M"), "2026-06-01T00:00:00.000Z");
        assert_eq!(eval("now/y"), "2026-01-01T00:00:00.000Z");
    }

    #[test]
    fn chained() {
        // The canonical Kibana bound: start of the day, one day ago.
        assert_eq!(eval("now-1d/d"), "2026-06-14T00:00:00.000Z");
        assert_eq!(eval("now-1M/M"), "2026-05-01T00:00:00.000Z");
    }

    #[test]
    fn anchored() {
        assert_eq!(
            eval("2026-01-15T10:00:00Z||+1d"),
            "2026-01-16T10:00:00.000Z"
        );
        assert_eq!(eval("2026-01-15T10:00:00Z||/M"), "2026-01-01T00:00:00.000Z");
    }

    #[test]
    fn month_arithmetic_crosses_years() {
        assert_eq!(eval("now-7M"), "2025-11-15T10:30:45.123Z");
        assert_eq!(eval("now+7M"), "2027-01-15T10:30:45.123Z");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_millis_at("not-a-date", Utc::now()).is_err());
        assert!(parse_millis_at("now-1x", Utc::now()).is_err());
        assert!(parse_millis_at("now~1d", Utc::now()).is_err());
    }
}
