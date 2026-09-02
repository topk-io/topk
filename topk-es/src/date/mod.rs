use std::fmt::{self, Display, Formatter};

use chrono::{
    DateTime, Datelike, Months, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, TimeDelta,
    TimeZone, Timelike, Utc, Weekday,
};
use chrono_tz::Tz;
use serde::Deserialize;
use topk_rs::proto::v1::control::{field_type, FieldSpec};
use topk_rs::proto::v1::data::Value;

use crate::Error;

mod interval;
pub use interval::{parse_calendar_interval, parse_fixed_interval, Interval};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Round {
    Down,
    Up,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
pub enum Zone {
    Fixed(chrono::FixedOffset),
    Named(Tz),
}

impl TryFrom<String> for Zone {
    type Error = Error;

    fn try_from(tz: String) -> Result<Self, Error> {
        if tz == "Z" {
            return Ok(Zone::UTC);
        }

        if let Ok(offset) = tz.parse::<chrono::FixedOffset>() {
            return Ok(Zone::Fixed(offset));
        }

        tz.parse::<Tz>()
            .map(Zone::Named)
            .map_err(|_| Error::BadRequest(format!("unknown time_zone [{tz}]")))
    }
}

impl Zone {
    pub const UTC: Zone = Zone::Named(Tz::UTC);

    fn local(&self, at: DateTime<Utc>) -> NaiveDateTime {
        match self {
            Zone::Fixed(offset) => at.with_timezone(offset).naive_local(),
            Zone::Named(tz) => at.with_timezone(tz).naive_local(),
        }
    }

    fn utc(&self, at: NaiveDateTime) -> Option<DateTime<Utc>> {
        fn resolve<T: TimeZone>(tz: &T, at: NaiveDateTime) -> Option<DateTime<Utc>> {
            (0..=49).find_map(|halves| {
                tz.from_local_datetime(&(at + TimeDelta::minutes(30 * halves)))
                    .earliest()
                    .map(|at| at.to_utc())
            })
        }

        match self {
            Zone::Fixed(offset) => resolve(offset, at),
            Zone::Named(tz) => resolve(tz, at),
        }
    }
}

impl Display for Zone {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Zone::Fixed(offset) => write!(f, "{offset}"),
            Zone::Named(tz) => write!(f, "{}", tz.name()),
        }
    }
}

pub fn format(millis: i64, zone: Option<&Zone>) -> Option<String> {
    fn fmt<T: TimeZone>(at: DateTime<Utc>, tz: &T) -> String
    where
        T::Offset: Display,
    {
        at.with_timezone(tz)
            .to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    let at = DateTime::<Utc>::from_timestamp_millis(millis)?;
    Some(match zone.unwrap_or(&Zone::UTC) {
        Zone::Fixed(offset) => fmt(at, offset),
        Zone::Named(tz) => fmt(at, tz),
    })
}

fn parse_millis_at(
    value: &str,
    zone: Option<&Zone>,
    round: Round,
    now: DateTime<Utc>,
) -> Result<i64, Error> {
    let zone = zone.copied().unwrap_or(Zone::UTC);

    if let Some(rest) = value.strip_prefix("now") {
        return millis(&zone, date_math(rest, round, zone.local(now))?);
    }

    if let Some((anchor, rest)) = value.split_once("||") {
        let (at, precision) = parse_instant(anchor, &zone)?;
        let at = zone.local(at);
        return match (rest.is_empty(), round) {
            (true, Round::Down) => millis(&zone, at),
            (true, Round::Up) => millis(&zone, end_of(at, precision)?),
            (false, _) => millis(&zone, date_math(rest, round, at)?),
        };
    }

    match parse_instant(value, &zone) {
        Ok((at, _)) if round == Round::Down => Ok(at.timestamp_millis()),
        Ok((at, precision)) => millis(&zone, end_of(zone.local(at), precision)?),
        Err(err) => value.parse::<i64>().map_err(|_| err),
    }
}

fn millis(zone: &Zone, at: NaiveDateTime) -> Result<i64, Error> {
    zone.utc(at)
        .map(|at| at.timestamp_millis())
        .ok_or_else(|| Error::BadRequest(format!("cannot apply time_zone [{zone}]")))
}

fn parse_instant(value: &str, zone: &Zone) -> Result<(DateTime<Utc>, Unit), Error> {
    let invalid = || Error::BadRequest(format!("cannot parse date [{value}]"));
    let precision = precision_of(value);

    if let Ok(at) = DateTime::parse_from_rfc3339(value) {
        return Ok((at.with_timezone(&Utc), precision));
    }

    let naive = [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
    ]
    .iter()
    .find_map(|fmt| NaiveDateTime::parse_from_str(value, fmt).ok())
    .or_else(|| {
        let ymd = match value.len() {
            4 => format!("{value}-01-01"),
            7 => format!("{value}-01"),
            _ => value.to_string(),
        };
        NaiveDate::parse_from_str(&ymd, "%Y-%m-%d")
            .ok()
            .map(|d| d.and_time(NaiveTime::MIN))
    })
    .ok_or_else(invalid)?;

    Ok((
        zone.utc(naive)
            .ok_or_else(|| Error::BadRequest(format!("cannot apply time_zone [{zone}]")))?,
        precision,
    ))
}

fn precision_of(value: &str) -> Unit {
    if value.contains('.') {
        return Unit::Millis;
    }
    let local = value.strip_suffix('Z').unwrap_or(value);
    let local = match local.get(10..).and_then(|tail| tail.find(['+', '-'])) {
        Some(i) => &local[..10 + i],
        None => local,
    };
    match local.len() {
        4 => Unit::Year,
        7 => Unit::Month,
        10 => Unit::Day,
        16 => Unit::Minute,
        _ => Unit::Second,
    }
}

fn date_math(expr: &str, round: Round, anchor: NaiveDateTime) -> Result<NaiveDateTime, Error> {
    let invalid = || Error::BadRequest(format!("cannot parse date [now{expr}]"));
    if !expr.is_ascii() {
        return Err(invalid());
    }
    let mut at = anchor;
    let mut rest = expr;

    while !rest.is_empty() {
        let (op, tail) = rest.split_at(1);
        match op {
            "/" => {
                let (unit, tail) = tail.split_at(1.min(tail.len()));
                let unit = unit_of(unit).ok_or_else(invalid)?;
                at = match round {
                    Round::Down => round_down(at, unit)?,
                    Round::Up => end_of(at, unit)?,
                };
                rest = tail;
            }
            "+" | "-" => {
                let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
                let (unit, tail) = tail[digits.len()..].split_at(1.min(tail.len() - digits.len()));
                let n: i64 = match digits.is_empty() {
                    true => 1,
                    false => digits.parse().map_err(|_| invalid())?,
                };
                at = add(
                    at,
                    unit_of(unit).ok_or_else(invalid)?,
                    if op == "-" { -n } else { n },
                )?;
                rest = tail;
            }
            _ => return Err(invalid()),
        }
    }

    Ok(at)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Millis,
    Year,
    Quarter,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
}

impl Unit {
    pub fn as_str(&self) -> &'static str {
        match self {
            Unit::Millis => "millisecond",
            Unit::Year => "year",
            Unit::Quarter => "quarter",
            Unit::Month => "month",
            Unit::Week => "week",
            Unit::Day => "day",
            Unit::Hour => "hour",
            Unit::Minute => "minute",
            Unit::Second => "second",
        }
    }
}

fn unit_of(unit: &str) -> Option<Unit> {
    Some(match unit {
        "y" => Unit::Year,
        "M" => Unit::Month,
        "w" => Unit::Week,
        "d" => Unit::Day,
        "H" | "h" => Unit::Hour,
        "m" => Unit::Minute,
        "s" => Unit::Second,
        _ => return None,
    })
}

pub(super) fn add(at: NaiveDateTime, unit: Unit, n: i64) -> Result<NaiveDateTime, Error> {
    let shift = |delta: Option<TimeDelta>| delta.and_then(|d| at.checked_add_signed(d));
    let months = |n: i64| {
        let months = Months::new(u32::try_from(n.unsigned_abs()).ok()?);
        match n < 0 {
            true => at.checked_sub_months(months),
            false => at.checked_add_months(months),
        }
    };

    match unit {
        Unit::Millis => shift(TimeDelta::try_milliseconds(n)),
        Unit::Year => n.checked_mul(12).and_then(months),
        Unit::Quarter => n.checked_mul(3).and_then(months),
        Unit::Month => months(n),
        Unit::Week => shift(TimeDelta::try_weeks(n)),
        Unit::Day => shift(TimeDelta::try_days(n)),
        Unit::Hour => shift(TimeDelta::try_hours(n)),
        Unit::Minute => shift(TimeDelta::try_minutes(n)),
        Unit::Second => shift(TimeDelta::try_seconds(n)),
    }
    .ok_or_else(|| Error::BadRequest("date math overflowed".to_string()))
}

fn round_down(at: NaiveDateTime, unit: Unit) -> Result<NaiveDateTime, Error> {
    let midnight = |d: NaiveDate| d.and_time(NaiveTime::MIN);

    match unit {
        Unit::Millis => Some(at),
        Unit::Year => at.date().with_ordinal(1).map(midnight),
        Unit::Quarter => at
            .date()
            .with_month((at.month() - 1) / 3 * 3 + 1)
            .and_then(|d| d.with_day(1))
            .map(midnight),
        Unit::Month => at.date().with_day(1).map(midnight),
        Unit::Week => Some(midnight(at.date().week(Weekday::Mon).first_day())),
        Unit::Day => Some(midnight(at.date())),
        Unit::Hour => at.with_minute(0).and_then(|d| d.with_second(0)),
        Unit::Minute => at.with_second(0),
        Unit::Second => Some(at),
    }
    .and_then(|d| d.with_nanosecond(0))
    .ok_or_else(|| Error::BadRequest("date rounding overflowed".to_string()))
}

fn end_of(at: NaiveDateTime, unit: Unit) -> Result<NaiveDateTime, Error> {
    let start = round_down(at, unit)?;
    add(start, unit, 1).and_then(|next| add(next, Unit::Millis, -1))
}

pub fn is_timestamp(spec: &FieldSpec) -> bool {
    matches!(
        spec.data_type.as_ref().and_then(|t| t.data_type.as_ref()),
        Some(field_type::DataType::Timestamp(_))
    )
}

pub fn to_timestamp(
    spec: Option<&FieldSpec>,
    value: Value,
    zone: Option<&Zone>,
    round: Round,
    now: DateTime<Utc>,
) -> Result<Value, Error> {
    if !spec.is_some_and(is_timestamp) {
        return Ok(value);
    }

    if let Some(s) = value.as_string() {
        return Ok(Value::timestamp(parse_millis_at(s, zone, round, now)?));
    }

    match value.as_string_list() {
        Some(values) => values
            .iter()
            .map(|v| parse_millis_at(v, zone, round, now))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::list),
        None => Ok(value),
    }
}

pub fn from_timestamp(value: Value) -> Value {
    match value.as_timestamp().and_then(|t| format(t, None)) {
        Some(iso) => Value::string(iso),
        None => Value::null(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[case(Unit::Quarter, 1, "2026-01-15T00:00:00", "2026-04-15T00:00:00")]
    #[case(Unit::Quarter, 3, "2026-01-15T00:00:00", "2026-10-15T00:00:00")]
    #[case(Unit::Quarter, -1, "2026-01-15T00:00:00", "2025-10-15T00:00:00")]
    #[case(Unit::Year, 1, "2026-01-15T00:00:00", "2027-01-15T00:00:00")]
    fn add_shifts_by_unit(
        #[case] unit: Unit,
        #[case] n: i64,
        #[case] at: &str,
        #[case] expected: &str,
    ) {
        let at: NaiveDateTime = at.parse().unwrap();
        assert_eq!(add(at, unit, n).unwrap(), expected.parse().unwrap());
    }

    #[rstest::rstest]
    #[case(Unit::Quarter, "2026-02-17T09:30:00", "2026-01-01T00:00:00")]
    #[case(Unit::Quarter, "2026-12-31T23:59:59", "2026-10-01T00:00:00")]
    #[case(Unit::Quarter, "2026-07-01T00:00:00", "2026-07-01T00:00:00")]
    fn round_down_snaps_to_unit_start(
        #[case] unit: Unit,
        #[case] at: &str,
        #[case] expected: &str,
    ) {
        let at: NaiveDateTime = at.parse().unwrap();
        assert_eq!(round_down(at, unit).unwrap(), expected.parse().unwrap());
    }

    #[test]
    fn date_math_rejects_non_ascii() {
        assert!(parse_millis_at("now/é", None, Round::Down, Utc::now()).is_err());
    }

    #[test]
    fn format_matches_elasticsearch() {
        assert_eq!(
            format(1797328800000, None).unwrap(),
            "2026-12-15T10:00:00.000Z"
        );
        assert_eq!(
            format(1797328800123, None).unwrap(),
            "2026-12-15T10:00:00.123Z"
        );

        let plus_two = Zone::try_from("+02:00".to_string()).unwrap();
        assert_eq!(
            format(1768435200000, Some(&plus_two)).unwrap(),
            "2026-01-15T02:00:00.000+02:00"
        );

        let london = Zone::try_from("Europe/London".to_string()).unwrap();
        assert_eq!(
            format(1768435200000, Some(&london)).unwrap(),
            "2026-01-15T00:00:00.000Z"
        );
    }

    #[test]
    fn zone_vocabulary() {
        for alias in ["Z", "UTC", "Etc/UTC", "GMT", "Etc/GMT"] {
            let zone = Zone::try_from(alias.to_string()).unwrap();
            assert!(matches!(zone, Zone::Named(_)), "{alias}");
            assert_eq!(format(0, Some(&zone)).unwrap(), "1970-01-01T00:00:00.000Z");
        }
        for offset in ["+02:00", "-08:00", "+0530"] {
            assert!(
                matches!(Zone::try_from(offset.to_string()), Ok(Zone::Fixed(_))),
                "{offset}"
            );
        }
        assert!(Zone::try_from("Mars/Olympus".to_string()).is_err());
    }
}
