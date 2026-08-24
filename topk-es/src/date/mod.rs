use chrono::{
    DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, Offset, SecondsFormat,
    TimeZone, Timelike, Utc,
};
use topk_rs::proto::v1::control::{field_type, FieldSpec};
use topk_rs::proto::v1::data::Value;

use crate::Error;

// Which end of the unit an under-specified bound resolves to. ES rounds a bound down under
// `gte`/`lt` and up under `gt`/`lte`, so `lte: "2026-06-10"` means the last millisecond of that
// day and `lte: <expr>||/w` the last millisecond of that week.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Round {
    Down,
    Up,
}

// `now` is threaded in rather than read here so date math is testable without the wall clock.
//
// All arithmetic and rounding happens in `tz`, as ES does: `2026-06-10||/d` under `-05:00` is
// local midnight (05:00Z), not UTC midnight.
fn parse_millis_at(
    value: &str,
    tz: Option<&str>,
    round: Round,
    now: DateTime<Utc>,
) -> Result<i64, Error> {
    let zone = tz.map(Zone::parse).transpose()?;
    let local = |at: DateTime<Utc>| match &zone {
        None => at.naive_utc(),
        Some(zone) => at.naive_utc() + Duration::milliseconds(zone.offset_millis(at)),
    };
    let millis = |naive: NaiveDateTime| match &zone {
        None => Ok(naive.and_utc().timestamp_millis()),
        Some(zone) => zone
            .from_local(naive)
            .map(|dt| dt.timestamp_millis())
            .ok_or_else(|| Error::BadRequest(format!("cannot apply time_zone [{}]", tz.unwrap()))),
    };

    if let Some(rest) = value.strip_prefix("now") {
        return millis(date_math(rest, round, local(now))?);
    }

    // ES anchors date math to an explicit date with `||`, e.g. `2026-01-15T00:00:00Z||+1d`.
    if let Some((anchor, rest)) = value.split_once("||") {
        let (anchor, precision) = parse_instant_precision(anchor, tz)?;
        let anchor = local(anchor);
        return match rest.is_empty() {
            // No math after `||`, so the anchor's own precision decides the rounding unit.
            true => millis(end_of(anchor, precision, round)),
            false => millis(date_math(rest, round, anchor)?),
        };
    }

    // Date formats win over epoch millis, as in ES's `strict_date_optional_time||epoch_millis`:
    // "2026" is the year 2026, not 2026ms into 1970.
    match parse_instant_precision(value, tz) {
        Ok((dt, precision)) => match round {
            // The instant is already correct; only an upward round needs local arithmetic.
            Round::Down => Ok(dt.timestamp_millis()),
            Round::Up => millis(end_of(local(dt), precision, round)),
        },
        // Epoch millis are exact, so rounding never applies.
        Err(err) => value.parse::<i64>().map_err(|_| err),
    }
}

// The last instant inside `unit` when rounding up, else the value unchanged.
fn end_of(at: NaiveDateTime, unit: Unit, round: Round) -> NaiveDateTime {
    match round {
        Round::Down => at,
        // `at` is already the start of its unit, so the unit's last millisecond is one unit on,
        // less 1ms.
        Round::Up => add(round_down(at, unit).unwrap_or(at), unit, 1)
            .map(|next| next - Duration::milliseconds(1))
            .unwrap_or(at),
    }
}

// A superset of what ES's default `strict_date_optional_time` accepts: a full RFC3339 stamp, a
// zone-less stamp, or a bare date. A value carrying its own offset wins; otherwise `time_zone`
// applies, defaulting to UTC. Declared mapping `format` patterns are not interpreted — being
// permissive here covers the common ones without threading formats through the schema.
// Also reports how precise the literal was: `2026-06-10` names a whole day, `2026-06` a month.
// ES uses that unit as the rounding granularity for an upward-rounded bound.
fn parse_instant_precision(
    value: &str,
    tz: Option<&str>,
) -> Result<(DateTime<Utc>, Unit), Error> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        // ES fills the unspecified part of a bound to its maximum, so a stamp written without
        // millis covers the whole second while one that spells them out is exact.
        let precision = match value.contains('.') {
            true => Unit::Millis,
            false => Unit::Second,
        };
        return Ok((dt.with_timezone(&Utc), precision));
    }

    let (naive, precision) = [
        ("%Y-%m-%dT%H:%M:%S%.f", Unit::Millis),
        ("%Y-%m-%dT%H:%M:%S", Unit::Second),
        ("%Y-%m-%dT%H:%M", Unit::Minute),
    ]
    .iter()
    .find_map(|(fmt, unit)| {
        NaiveDateTime::parse_from_str(value, fmt)
            .ok()
            .map(|dt| (dt, *unit))
    })
    .or_else(|| {
        // `strict_date_optional_time` also accepts a bare date, year-month, or year.
        let (ymd, unit) = match value.len() {
            4 => (format!("{value}-01-01"), Unit::Year),
            7 => (format!("{value}-01"), Unit::Month),
            _ => (value.to_string(), Unit::Day),
        };
        NaiveDate::parse_from_str(&ymd, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| (dt, unit))
    })
    .ok_or_else(|| Error::BadRequest(format!("cannot parse date [{value}]")))?;

    let at = match tz {
        None => naive.and_utc(),
        Some(tz) => Zone::parse(tz)?
            .from_local(naive)
            .ok_or_else(|| Error::BadRequest(format!("cannot apply time_zone [{tz}]")))?,
    };
    Ok((at, precision))
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
            Zone::Named(tz) => tz
                .offset_from_utc_datetime(&at.naive_utc())
                .fix()
                .local_minus_utc(),
        };
        seconds as i64 * 1_000
    }
}

// ES date math: a chain of `+Nunit` / `-Nunit` offsets and `/unit` roundings, e.g. `-1d/d`.
fn date_math(expr: &str, round: Round, anchor: NaiveDateTime) -> Result<NaiveDateTime, Error> {
    let mut at = anchor;
    let mut rest = expr;

    while !rest.is_empty() {
        let (op, tail) = rest.split_at(1);
        match op {
            "/" => {
                let (unit, tail) = tail.split_at(1.min(tail.len()));
                let unit = unit_of(unit, expr)?;
                // `/unit` snaps to the unit's start, or its last instant when rounding up.
                at = end_of(round_down(at, unit)?, unit, round);
                rest = tail;
            }
            "+" | "-" => {
                let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
                let (unit, tail) = tail[digits.len()..].split_at(1.min(tail.len() - digits.len()));
                let n: i64 = match digits.is_empty() {
                    // An omitted count means 1 (`now+y`); an unparseable one is an error, not 1.
                    true => 1,
                    false => digits
                        .parse()
                        .map_err(|_| Error::BadRequest(format!("cannot parse date [now{expr}]")))?,
                };
                let n = if op == "-" { -n } else { n };
                at = add(at, unit_of(unit, expr)?, n)?;
                rest = tail;
            }
            _ => return Err(Error::BadRequest(format!("cannot parse date [now{expr}]"))),
        }
    }

    Ok(at)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    // A value specified to the millisecond is exact, so rounding it is a no-op.
    Millis,
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

fn add(at: NaiveDateTime, unit: Unit, n: i64) -> Result<NaiveDateTime, Error> {
    let shifted = match unit {
        Unit::Millis => Some(at + Duration::milliseconds(n)),
        // Calendar units vary in length, so they shift the date rather than add a fixed duration.
        Unit::Year => add_months(at, n * 12),
        Unit::Month => add_months(at, n),
        Unit::Week => Some(at + Duration::weeks(n)),
        Unit::Day => Some(at + Duration::days(n)),
        Unit::Hour => Some(at + Duration::hours(n)),
        Unit::Minute => Some(at + Duration::minutes(n)),
        Unit::Second => Some(at + Duration::seconds(n)),
    };

    shifted.ok_or_else(|| Error::BadRequest("date math overflowed".to_string()))
}

// ES clamps to the last day of the target month: Jan 31 + 1M is Feb 28, Feb 29 + 1y is Feb 28.
fn add_months(at: NaiveDateTime, n: i64) -> Option<NaiveDateTime> {
    let months = at.year() as i64 * 12 + at.month0() as i64 + n;
    let (year, month) = (
        months.div_euclid(12) as i32,
        (months.rem_euclid(12) + 1) as u32,
    );
    let day = (28..=at.day())
        .rev()
        .find(|day| NaiveDate::from_ymd_opt(year, month, *day).is_some())
        .unwrap_or(at.day());

    NaiveDate::from_ymd_opt(year, month, day)?
        .and_hms_nano_opt(at.hour(), at.minute(), at.second(), at.nanosecond())
}

// `/unit` rounds down to the start of that unit, as ES does.
fn round_down(at: NaiveDateTime, unit: Unit) -> Result<NaiveDateTime, Error> {
    let midnight = |d: NaiveDate| d.and_hms_opt(0, 0, 0);
    let rounded = match unit {
        Unit::Millis => return Ok(at),
        Unit::Year => NaiveDate::from_ymd_opt(at.year(), 1, 1).and_then(midnight),
        Unit::Month => NaiveDate::from_ymd_opt(at.year(), at.month(), 1).and_then(midnight),
        Unit::Week => midnight(at.date().week(chrono::Weekday::Mon).first_day()),
        Unit::Day => midnight(at.date()),
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

// ES renders a date_histogram `key_as_string` in the request's `time_zone`, offset notation and
// all; a named zone's offset is whatever held at that instant.
pub fn format_key(millis: i64, zone: Option<&Zone>) -> Option<String> {
    let dt = DateTime::<Utc>::from_timestamp_millis(millis)?;
    Some(match zone {
        None => dt.to_rfc3339_opts(SecondsFormat::Millis, true),
        Some(Zone::Fixed(offset)) => dt
            .with_timezone(offset)
            .to_rfc3339_opts(SecondsFormat::Millis, true),
        Some(Zone::Named(tz)) => dt
            .with_timezone(tz)
            .to_rfc3339_opts(SecondsFormat::Millis, true),
    })
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
    to_timestamp_rounded(spec, value, tz, Round::Down)
}

// As `to_timestamp`, but for a range bound, where an under-specified value resolves to the end
// of its unit under `gt`/`lte`.
pub fn to_timestamp_rounded(
    spec: Option<&FieldSpec>,
    value: Value,
    tz: Option<&str>,
    round: Round,
) -> Result<Value, Error> {
    if !spec.is_some_and(is_timestamp) {
        return Ok(value);
    }

    let parse = |v: &str| parse_millis_at(v, tz, round, Utc::now());

    if let Some(s) = value.as_string() {
        return Ok(Value::timestamp(parse(s)?));
    }

    match value.as_string_list() {
        Some(values) => values
            .iter()
            .map(|v| parse(v))
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
pub use interval::{bucketing, Bucketing};

#[cfg(test)]
mod tests {
    use super::*;

    fn at(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn eval(expr: &str) -> String {
        let now = at("2026-06-15T10:30:45.123Z");
        format_millis(parse_millis_at(expr, None, Round::Down, now).expect(expr)).unwrap()
    }

    // The upper end of the unit an under-specified bound names, as `lte`/`gt` resolve it.
    fn eval_up(expr: &str) -> String {
        let now = at("2026-06-15T10:30:45.123Z");
        format_millis(parse_millis_at(expr, None, Round::Up, now).expect(expr)).unwrap()
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
    fn month_arithmetic_clamps_to_month_end() {
        assert_eq!(
            eval("2026-05-31T00:00:00Z||-1M"),
            "2026-04-30T00:00:00.000Z"
        );
        assert_eq!(
            eval("2026-01-31T00:00:00Z||+1M"),
            "2026-02-28T00:00:00.000Z"
        );
        assert_eq!(
            eval("2028-02-29T00:00:00Z||+1y"),
            "2029-02-28T00:00:00.000Z"
        );
    }

    #[test]
    fn bare_year_and_year_month() {
        assert_eq!(eval("2026"), "2026-01-01T00:00:00.000Z");
        assert_eq!(eval("2026-03"), "2026-03-01T00:00:00.000Z");
    }

    #[test]
    fn accepts_common_shapes() {
        assert_eq!(eval("2026-01-15"), "2026-01-15T00:00:00.000Z");
        assert_eq!(eval("2026-01-15T10:00:00"), "2026-01-15T10:00:00.000Z");
        assert_eq!(eval("2026-01-15T10:00"), "2026-01-15T10:00:00.000Z");
        assert_eq!(eval("2026-01-15T10:00:00.500"), "2026-01-15T10:00:00.500Z");
        assert_eq!(
            eval("2026-01-15T10:00:00+02:00"),
            "2026-01-15T08:00:00.000Z"
        );
    }

    #[test]
    fn time_zone_applies_to_zoneless_values_only() {
        let now = at("2026-06-15T10:30:45.123Z");
        let tz =
            |v: &str, tz: &str| format_millis(parse_millis_at(v, Some(tz), Round::Down, now).expect(v)).unwrap();

        assert_eq!(tz("2026-01-15", "+02:00"), "2026-01-14T22:00:00.000Z");
        assert_eq!(
            tz("2026-01-15T00:00:00", "-05:00"),
            "2026-01-15T05:00:00.000Z"
        );
        // An explicit offset on the value wins over `time_zone`.
        assert_eq!(
            tz("2026-01-15T00:00:00Z", "+02:00"),
            "2026-01-15T00:00:00.000Z"
        );
        assert_eq!(tz("2026-01-15", "Z"), "2026-01-15T00:00:00.000Z");
    }

    #[test]
    fn named_time_zones() {
        let now = at("2026-06-15T10:30:45.123Z");
        let tz =
            |v: &str, tz: &str| format_millis(parse_millis_at(v, Some(tz), Round::Down, now).expect(v)).unwrap();

        // Prague is +01:00 in winter and +02:00 under DST.
        assert_eq!(
            tz("2026-01-15T00:00:00", "Europe/Prague"),
            "2026-01-14T23:00:00.000Z"
        );
        assert_eq!(
            tz("2026-07-15T00:00:00", "Europe/Prague"),
            "2026-07-14T22:00:00.000Z"
        );
        assert_eq!(tz("2026-01-15T00:00:00", "UTC"), "2026-01-15T00:00:00.000Z");
    }

    #[test]
    fn rejects_unknown_time_zone() {
        assert!(parse_millis_at("2026-01-15", Some("Mars/Olympus"), Round::Down, Utc::now()).is_err());
    }

    #[test]
    fn rounds_bounds_up_to_end_of_unit() {
        // Verified against Elasticsearch 9: an upward-rounded bound is the unit's last millis.
        assert_eq!(eval_up("2026-06-10"), "2026-06-10T23:59:59.999Z");
        assert_eq!(eval_up("2026-06"), "2026-06-30T23:59:59.999Z");
        assert_eq!(eval_up("2026"), "2026-12-31T23:59:59.999Z");
        assert_eq!(eval_up("2026-06-08T00:00:00Z||/w"), "2026-06-14T23:59:59.999Z");
        assert_eq!(eval_up("now/d"), "2026-06-15T23:59:59.999Z");
        // ES fills the unspecified millis of a seconds-precision bound to 999.
        assert_eq!(eval_up("2026-06-10T10:00:00Z"), "2026-06-10T10:00:00.999Z");
        // Spelled-out millis are exact.
        assert_eq!(eval_up("2026-06-10T10:00:00.000Z"), "2026-06-10T10:00:00.000Z");
        assert_eq!(eval_up("1768471200000"), "2026-01-15T10:00:00.000Z");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_millis_at("not-a-date", None, Round::Down, Utc::now()).is_err());
        assert!(parse_millis_at("now-1x", None, Round::Down, Utc::now()).is_err());
        assert!(parse_millis_at("now~1d", None, Round::Down, Utc::now()).is_err());
        assert!(parse_millis_at("now+99999999999999999999d", None, Round::Down, Utc::now()).is_err());
    }
}
