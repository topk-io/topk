use std::fmt::{self, Display, Formatter};

use chrono::{
    DateTime, Datelike, FixedOffset, Months, NaiveDate, NaiveDateTime, NaiveTime, Offset,
    SecondsFormat, TimeDelta, TimeZone, Timelike, Utc, Weekday,
};
use chrono_tz::Tz;
use serde::Deserialize;
use topk_rs::proto::v1::control::{field_type, FieldSpec};
use topk_rs::proto::v1::data::logical_expr::Interval;
use topk_rs::proto::v1::data::{LogicalExpr, Value};

use crate::Error;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Round {
    Down,
    Up,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Zone {
    Fixed(FixedOffset),
    Named(Tz),
}

impl Default for Zone {
    fn default() -> Self {
        Zone::Named(Tz::UTC)
    }
}

impl TryFrom<String> for Zone {
    type Error = Error;

    fn try_from(tz: String) -> Result<Self, Error> {
        if tz == "Z" {
            return Ok(Zone::default());
        }

        if let Ok(offset) = tz.parse::<FixedOffset>() {
            return Ok(Zone::Fixed(offset));
        }

        tz.parse::<Tz>()
            .map(Zone::Named)
            .map_err(|_| Error::BadRequest(format!("unknown time_zone [{tz}]")))
    }
}

impl Zone {
    fn offset(&self, at: DateTime<Utc>) -> FixedOffset {
        match self {
            Zone::Fixed(offset) => *offset,
            Zone::Named(tz) => tz.offset_from_utc_datetime(&at.naive_utc()).fix(),
        }
    }

    pub(crate) fn fixed_offset_millis(&self) -> Option<i64> {
        match self {
            Zone::Fixed(offset) => Some(offset.local_minus_utc() as i64 * 1_000),
            Zone::Named(tz) if *tz == Tz::UTC => Some(0),
            Zone::Named(_) => None,
        }
    }

    pub(crate) fn local(&self, at: DateTime<Utc>) -> NaiveDateTime {
        at.with_timezone(&self.offset(at)).naive_local()
    }

    pub(crate) fn utc(&self, at: NaiveDateTime) -> Option<DateTime<Utc>> {
        match self {
            Zone::Fixed(offset) => at.checked_sub_offset(*offset).map(|at| at.and_utc()),
            Zone::Named(tz) => (0..=49).find_map(|halves| {
                tz.from_local_datetime(&(at + TimeDelta::minutes(30 * halves)))
                    .earliest()
                    .map(|at| at.to_utc())
            }),
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

pub fn format(millis: i64, zone: &Zone) -> Option<String> {
    let at = DateTime::<Utc>::from_timestamp_millis(millis)?;
    Some(
        at.with_timezone(&zone.offset(at))
            .to_rfc3339_opts(SecondsFormat::Millis, true),
    )
}

pub fn to_expr(
    spec: Option<&FieldSpec>,
    value: Value,
    zone: &Zone,
    round: Round,
) -> Result<LogicalExpr, Error> {
    match value.as_string().filter(|_| spec.is_some_and(is_timestamp)) {
        Some(s) => date_expr(s, zone, round),
        None => Ok(LogicalExpr::literal(value)),
    }
}

pub fn date_expr(value: &str, zone: &Zone, round: Round) -> Result<LogicalExpr, Error> {
    let (anchor, math) = match value.split_once("||") {
        Some(split) => split,
        None if value.starts_with("now") => ("now", &value[3..]),
        None => (value, ""),
    };
    let (mut expr, precision) = match anchor {
        "now" => (LogicalExpr::now(), Unit::Millisecond),
        _ => {
            let (at, precision) = parse_instant(anchor, zone)?;
            let at = LogicalExpr::literal(Value::timestamp(at.timestamp_millis()));
            (at, precision)
        }
    };

    if math.is_empty() {
        return match round {
            Round::Down => Ok(expr),
            Round::Up => end_of(expr, precision, zone),
        };
    }

    let invalid = || Error::BadRequest(format!("cannot parse date math [{math}]"));
    let mut chars = math.chars().peekable();
    while let Some(op) = chars.next() {
        let digits: String = std::iter::from_fn(|| chars.next_if(char::is_ascii_digit)).collect();
        let n: i64 = match digits.is_empty() {
            true => 1,
            false => digits.parse().map_err(|_| invalid())?,
        };
        let unit: Unit = chars
            .next()
            .and_then(|c| c.to_string().parse().ok())
            .ok_or_else(invalid)?;

        expr = match (op, round) {
            ('/', Round::Down) => expr.date_trunc(unit.to_string(), zone.to_string()),
            ('/', Round::Up) => end_of(expr, unit, zone)?,
            ('+', _) => expr.date_add(interval(unit, n)?, zone.to_string()),
            ('-', _) => expr.date_add(interval(unit, -n)?, zone.to_string()),
            _ => return Err(invalid()),
        };
    }

    Ok(expr)
}

fn end_of(expr: LogicalExpr, unit: Unit, zone: &Zone) -> Result<LogicalExpr, Error> {
    if unit == Unit::Millisecond {
        return Ok(expr);
    }
    let zone = zone.to_string();
    Ok(expr
        .date_trunc(unit.to_string(), zone.clone())
        .date_add(interval(unit, 1)?, zone.clone())
        .date_add(interval(Unit::Millisecond, -1)?, zone))
}

fn interval(unit: Unit, n: i64) -> Result<Interval, Error> {
    let overflow = || Error::BadRequest("date math overflowed".to_string());
    let months = |per: i64| {
        let months = n.checked_mul(per).and_then(|m| i32::try_from(m).ok());
        months.map(|months| Interval {
            months,
            ..Default::default()
        })
    };
    let days = |per: i64| {
        let days = n.checked_mul(per).and_then(|d| i32::try_from(d).ok());
        days.map(|days| Interval {
            days,
            ..Default::default()
        })
    };
    let millis = |per: i64| {
        n.checked_mul(per).map(|millis| Interval {
            millis,
            ..Default::default()
        })
    };

    match unit {
        Unit::Year => months(12),
        Unit::Quarter => months(3),
        Unit::Month => months(1),
        Unit::Week => days(7),
        Unit::Day => days(1),
        Unit::Hour => millis(3_600_000),
        Unit::Minute => millis(60_000),
        Unit::Second => millis(1_000),
        Unit::Millisecond => millis(1),
    }
    .ok_or_else(overflow)
}

fn parse_instant(value: &str, zone: &Zone) -> Result<(DateTime<Utc>, Unit), Error> {
    let invalid = || Error::BadRequest(format!("cannot parse date [{value}]"));

    if let Ok(at) = DateTime::parse_from_rfc3339(value) {
        let precision = match value.contains('.') {
            true => Unit::Millisecond,
            false => Unit::Second,
        };
        return Ok((at.to_utc(), precision));
    }

    let parsed = [
        ("%Y-%m-%dT%H:%M:%S", Unit::Second),
        ("%Y-%m-%dT%H:%M:%S%.f", Unit::Millisecond),
        ("%Y-%m-%dT%H:%M", Unit::Minute),
    ]
    .into_iter()
    .find_map(|(fmt, unit)| Some((NaiveDateTime::parse_from_str(value, fmt).ok()?, unit)))
    .or_else(|| {
        let (ymd, unit) = match value.len() {
            4 => (format!("{value}-01-01"), Unit::Year),
            7 => (format!("{value}-01"), Unit::Month),
            _ => (value.to_string(), Unit::Day),
        };
        let date = NaiveDate::parse_from_str(&ymd, "%Y-%m-%d").ok()?;
        Some((date.and_time(NaiveTime::MIN), unit))
    });

    match parsed {
        Some((local, precision)) => Ok((
            zone.utc(local)
                .ok_or_else(|| Error::BadRequest(format!("cannot apply time_zone [{zone}]")))?,
            precision,
        )),
        None => {
            let at = value
                .parse::<i64>()
                .ok()
                .and_then(DateTime::from_timestamp_millis);
            Ok((at.ok_or_else(invalid)?, Unit::Millisecond))
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumString)]
pub enum Unit {
    #[strum(to_string = "millisecond")]
    Millisecond,
    #[strum(to_string = "year", serialize = "1y", serialize = "y")]
    Year,
    #[strum(to_string = "quarter", serialize = "1q")]
    Quarter,
    #[strum(to_string = "month", serialize = "1M", serialize = "M")]
    Month,
    #[strum(to_string = "week", serialize = "1w", serialize = "w")]
    Week,
    #[strum(to_string = "day", serialize = "1d", serialize = "d")]
    Day,
    #[strum(to_string = "hour", serialize = "1h", serialize = "h", serialize = "H")]
    Hour,
    #[strum(to_string = "minute", serialize = "1m", serialize = "m")]
    Minute,
    #[strum(to_string = "second", serialize = "s")]
    Second,
}

pub(crate) fn add(at: NaiveDateTime, unit: Unit, n: i64) -> Result<NaiveDateTime, Error> {
    let shift = |delta: Option<TimeDelta>| delta.and_then(|d| at.checked_add_signed(d));
    let months = |n: i64| {
        let months = Months::new(u32::try_from(n.unsigned_abs()).ok()?);
        match n < 0 {
            true => at.checked_sub_months(months),
            false => at.checked_add_months(months),
        }
    };

    match unit {
        Unit::Millisecond => shift(TimeDelta::try_milliseconds(n)),
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

pub(crate) fn round_down(at: NaiveDateTime, unit: Unit) -> Result<NaiveDateTime, Error> {
    let midnight = |d: NaiveDate| d.and_time(NaiveTime::MIN);

    match unit {
        Unit::Millisecond => Some(at),
        Unit::Year => at.date().with_ordinal(1).map(midnight),
        Unit::Quarter => at
            .date()
            .with_day(1)
            .and_then(|d| d.with_month((at.month() - 1) / 3 * 3 + 1))
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

pub fn is_timestamp(spec: &FieldSpec) -> bool {
    matches!(
        spec.data_type.as_ref().and_then(|t| t.data_type.as_ref()),
        Some(field_type::DataType::Timestamp(_))
    )
}

pub fn millis(value: Value, zone: &Zone) -> Result<i64, Error> {
    match value.as_string() {
        Some(s) => Ok(parse_instant(s, zone)?.0.timestamp_millis()),
        None => value
            .as_timestamp()
            .ok_or_else(|| Error::BadRequest(format!("cannot parse date [{value:?}]"))),
    }
}

pub fn parse_date(value: Value) -> Result<Value, Error> {
    let parse = |s: &str| parse_instant(s, &Zone::default()).map(|(at, _)| at.timestamp_millis());
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

pub fn from_timestamp(value: Value) -> Value {
    match value
        .as_timestamp()
        .and_then(|t| format(t, &Zone::default()))
    {
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
    #[case(Unit::Quarter, "2026-05-31T12:00:00", "2026-04-01T00:00:00")]
    fn round_down_snaps_to_unit_start(
        #[case] unit: Unit,
        #[case] at: &str,
        #[case] expected: &str,
    ) {
        let at: NaiveDateTime = at.parse().unwrap();
        assert_eq!(round_down(at, unit).unwrap(), expected.parse().unwrap());
    }

    #[rstest::rstest]
    #[case("2026-01-15T10:00:00", Unit::Second)]
    #[case("2026-01-15T10:00:00.123", Unit::Millisecond)]
    #[case("2026-01-15T10:00", Unit::Minute)]
    #[case("2026-01-15", Unit::Day)]
    #[case("2026-01", Unit::Month)]
    #[case("2026", Unit::Year)]
    fn parse_instant_reports_precision(#[case] value: &str, #[case] expected: Unit) {
        assert_eq!(
            parse_instant(value, &Zone::default())
                .unwrap()
                .1
                .to_string(),
            expected.to_string()
        );
    }

    #[test]
    fn date_math_rejects_garbage() {
        for value in ["now/é", "now+1x", "now*1d", "not-a-date", "2026-01-01||1d"] {
            assert!(
                date_expr(value, &Zone::default(), Round::Down).is_err(),
                "{value}"
            );
        }
    }

    #[test]
    fn format_matches_elasticsearch() {
        let utc = Zone::default();
        assert_eq!(
            format(1797328800000, &utc).unwrap(),
            "2026-12-15T10:00:00.000Z"
        );
        assert_eq!(
            format(1797328800123, &utc).unwrap(),
            "2026-12-15T10:00:00.123Z"
        );

        let plus_two = Zone::try_from("+02:00".to_string()).unwrap();
        assert_eq!(
            format(1768435200000, &plus_two).unwrap(),
            "2026-01-15T02:00:00.000+02:00"
        );

        let london = Zone::try_from("Europe/London".to_string()).unwrap();
        assert_eq!(
            format(1768435200000, &london).unwrap(),
            "2026-01-15T00:00:00.000Z"
        );
    }

    #[test]
    fn zone_vocabulary() {
        for alias in ["Z", "UTC", "Etc/UTC", "GMT", "Etc/GMT"] {
            let zone = Zone::try_from(alias.to_string()).unwrap();
            assert!(matches!(zone, Zone::Named(_)), "{alias}");
            assert_eq!(format(0, &zone).unwrap(), "1970-01-01T00:00:00.000Z");
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
