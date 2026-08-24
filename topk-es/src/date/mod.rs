use chrono::{
    DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, Offset, SecondsFormat,
    TimeZone, Timelike, Utc,
};
use topk_rs::proto::v1::control::{field_type, FieldSpec};
use topk_rs::proto::v1::data::Value;

use crate::Error;

// ES `date` values arrive as ISO-8601 strings (or already-epoch millis) and must land in TopK's
// timestamp column as i64 millis; reads go the other way.
fn parse_millis(value: &str, tz: Option<&str>) -> Result<i64, Error> {
    parse_millis_at(value, tz, Utc::now())
}

// `now` is threaded in rather than read here so date math is testable without the wall clock.
fn parse_millis_at(value: &str, tz: Option<&str>, now: DateTime<Utc>) -> Result<i64, Error> {
    if let Ok(millis) = value.parse::<i64>() {
        return Ok(millis);
    }

    if let Some(rest) = value.strip_prefix("now") {
        return date_math(rest, now);
    }

    // ES anchors date math to an explicit date with `||`, e.g. `2026-01-15T00:00:00Z||+1d`.
    if let Some((anchor, rest)) = value.split_once("||") {
        return date_math(rest, parse_instant(anchor, tz)?);
    }

    parse_instant(value, tz).map(|dt| dt.timestamp_millis())
}

// A superset of what ES's default `strict_date_optional_time` accepts: a full RFC3339 stamp, a
// zone-less stamp, or a bare date. A value carrying its own offset wins; otherwise `time_zone`
// applies, defaulting to UTC. Declared mapping `format` patterns are not interpreted — being
// permissive here covers the common ones without threading formats through the schema.
fn parse_instant(value: &str, tz: Option<&str>) -> Result<DateTime<Utc>, Error> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Ok(dt.with_timezone(&Utc));
    }

    let naive = ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M"]
        .iter()
        .find_map(|fmt| NaiveDateTime::parse_from_str(value, fmt).ok())
        .or_else(|| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
        })
        .ok_or_else(|| Error::BadRequest(format!("cannot parse date [{value}]")))?;

    match tz {
        None => Ok(naive.and_utc()),
        Some(tz) => Zone::parse(tz)?
            .from_local(naive)
            .ok_or_else(|| Error::BadRequest(format!("cannot apply time_zone [{tz}]"))),
    }
}

// A `time_zone`, either a numeric offset (`+02:00`) or an IANA name (`Europe/Prague`). Named
// zones observe DST, so the offset depends on the instant being converted.
pub enum Zone {
    Fixed(FixedOffset),
    Named(chrono_tz::Tz),
}

impl Zone {
    pub fn parse(tz: &str) -> Result<Self, Error> {
        if tz == "Z" {
            return Ok(Zone::Fixed(FixedOffset::east_opt(0).expect("valid offset")));
        }

        if let Ok(dt) = DateTime::parse_from_rfc3339(&format!("1970-01-01T00:00:00{tz}")) {
            return Ok(Zone::Fixed(*dt.offset()));
        }

        tz.parse::<chrono_tz::Tz>()
            .map(Zone::Named)
            .map_err(|_| Error::BadRequest(format!("unknown time_zone [{tz}]")))
    }

    fn from_local(&self, naive: NaiveDateTime) -> Option<DateTime<Utc>> {
        match self {
            Zone::Fixed(offset) => offset
                .from_local_datetime(&naive)
                .single()
                .map(|dt| dt.with_timezone(&Utc)),
            Zone::Named(tz) => tz
                .from_local_datetime(&naive)
                .single()
                .map(|dt| dt.with_timezone(&Utc)),
        }
    }

    // The zone's UTC offset in milliseconds at `at`. A named zone's offset moves with DST, so this
    // is only a constant for as long as the queried span stays on one side of a transition.
    pub fn offset_millis(&self, at: DateTime<Utc>) -> i64 {
        let seconds = match self {
            Zone::Fixed(offset) => offset.local_minus_utc(),
            Zone::Named(tz) => tz.offset_from_utc_datetime(&at.naive_utc()).fix().local_minus_utc(),
        };
        seconds as i64 * 1_000
    }
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
pub fn to_timestamp(
    spec: Option<&FieldSpec>,
    value: Value,
    tz: Option<&str>,
) -> Result<Value, Error> {
    if !spec.is_some_and(is_timestamp) {
        return Ok(value);
    }

    if let Some(s) = value.as_string() {
        return Ok(Value::timestamp(parse_millis(s, tz)?));
    }

    match value.as_string_list() {
        Some(values) => values
            .iter()
            .map(|v| parse_millis(v, tz))
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

mod interval;
pub use interval::{bucket_key, bucket_start, interval, offset_millis};

#[cfg(test)]
mod tests {
    use super::*;

    fn at(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso).unwrap().with_timezone(&Utc)
    }

    fn eval(expr: &str) -> String {
        let now = at("2026-06-15T10:30:45.123Z");
        format_millis(parse_millis_at(expr, None, now).expect(expr)).unwrap()
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
    fn accepts_common_shapes() {
        assert_eq!(eval("2026-01-15"), "2026-01-15T00:00:00.000Z");
        assert_eq!(eval("2026-01-15T10:00:00"), "2026-01-15T10:00:00.000Z");
        assert_eq!(eval("2026-01-15T10:00"), "2026-01-15T10:00:00.000Z");
        assert_eq!(eval("2026-01-15T10:00:00.500"), "2026-01-15T10:00:00.500Z");
        assert_eq!(eval("2026-01-15T10:00:00+02:00"), "2026-01-15T08:00:00.000Z");
    }

    #[test]
    fn time_zone_applies_to_zoneless_values_only() {
        let now = at("2026-06-15T10:30:45.123Z");
        let tz = |v: &str, tz: &str| {
            format_millis(parse_millis_at(v, Some(tz), now).expect(v)).unwrap()
        };

        assert_eq!(tz("2026-01-15", "+02:00"), "2026-01-14T22:00:00.000Z");
        assert_eq!(tz("2026-01-15T00:00:00", "-05:00"), "2026-01-15T05:00:00.000Z");
        // An explicit offset on the value wins over `time_zone`.
        assert_eq!(tz("2026-01-15T00:00:00Z", "+02:00"), "2026-01-15T00:00:00.000Z");
        assert_eq!(tz("2026-01-15", "Z"), "2026-01-15T00:00:00.000Z");
    }

    #[test]
    fn named_time_zones() {
        let now = at("2026-06-15T10:30:45.123Z");
        let tz = |v: &str, tz: &str| {
            format_millis(parse_millis_at(v, Some(tz), now).expect(v)).unwrap()
        };

        // Prague is +01:00 in winter and +02:00 under DST.
        assert_eq!(tz("2026-01-15T00:00:00", "Europe/Prague"), "2026-01-14T23:00:00.000Z");
        assert_eq!(tz("2026-07-15T00:00:00", "Europe/Prague"), "2026-07-14T22:00:00.000Z");
        assert_eq!(tz("2026-01-15T00:00:00", "UTC"), "2026-01-15T00:00:00.000Z");
    }

    #[test]
    fn rejects_unknown_time_zone() {
        assert!(parse_millis_at("2026-01-15", Some("Mars/Olympus"), Utc::now()).is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_millis_at("not-a-date", None, Utc::now()).is_err());
        assert!(parse_millis_at("now-1x", None, Utc::now()).is_err());
        assert!(parse_millis_at("now~1d", None, Utc::now()).is_err());
    }
}
