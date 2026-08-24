use chrono::{DateTime, Datelike, Days, Duration, LocalResult, NaiveDate, TimeZone, Utc, Weekday};
use topk_rs::proto::v1::data::LogicalExpr;
use topk_rs::query::field;

use super::{Error, Zone};
use crate::api::DateHistogramBody;

const HOUR: i64 = 60 * 60 * 1_000;
const DAY: i64 = 24 * HOUR;

// How a date_histogram maps documents to buckets. `Shifted`/`Months` bucket entirely in the
// engine on `ts + offset`, one engine row per bucket. A named `time_zone` observes DST, so no
// constant offset is correct for calendar days and up; `Local` has the engine bucket by a
// granularity finer than any zone transition and merges the rows into local-calendar buckets
// here, where chrono-tz can follow the zone.
pub enum Bucketing {
    Shifted {
        width: i64,
        offset: i64,
    },
    Months {
        count: i64,
        offset: i64,
    },
    Local {
        unit: LocalUnit,
        tz: chrono_tz::Tz,
        granularity: i64,
    },
}

#[derive(Clone, Copy)]
pub enum LocalUnit {
    Day,
    Week,
    Months(i64),
}

// A parsed interval, still carrying whether it is calendar-shaped: `fixed_interval: 7d` buckets
// a flat week of millis, while `calendar_interval: week` follows local Mondays under a zone.
enum Parsed {
    Fixed(i64),
    Day,
    Week,
    Months(i64),
}

pub fn bucketing(h: &DateHistogramBody) -> Result<Bucketing, Error> {
    let parsed = match (&h.fixed_interval, &h.calendar_interval) {
        (Some(_), Some(_)) => Err(Error::BadRequest(
            "date_histogram accepts either fixed_interval or calendar_interval, not both".into(),
        )),
        (Some(fixed), None) => fixed_interval(fixed).map(Parsed::Fixed),
        (None, Some(calendar)) => calendar_interval(calendar),
        (None, None) => Err(Error::BadRequest(
            "date_histogram requires fixed_interval or calendar_interval".into(),
        )),
    }?;
    let zone = h.time_zone.as_deref().map(Zone::parse).transpose()?;

    // The epoch is a Thursday; pre-shifting by 3 days lands week buckets on Mondays, as ES does.
    const WEEK_SHIFT: i64 = 3 * DAY;

    // A fixed interval keeps its exact width even under a named zone, as in ES; the zone only
    // shifts alignment, resolved once at `now` (exact for fixed offsets, approximate within an
    // hour of a DST transition for named ones).
    let offset = match &zone {
        None => 0,
        Some(zone) => zone.offset_millis(Utc::now()),
    };

    match (parsed, zone) {
        (Parsed::Fixed(width), _) => Ok(Bucketing::Shifted { width, offset }),
        (Parsed::Day, Some(Zone::Named(tz))) => Ok(local(LocalUnit::Day, tz)),
        (Parsed::Week, Some(Zone::Named(tz))) => Ok(local(LocalUnit::Week, tz)),
        (Parsed::Months(n), Some(Zone::Named(tz))) => Ok(local(LocalUnit::Months(n), tz)),
        (Parsed::Day, _) => Ok(Bucketing::Shifted { width: DAY, offset }),
        (Parsed::Week, _) => Ok(Bucketing::Shifted {
            width: 7 * DAY,
            offset: offset + WEEK_SHIFT,
        }),
        (Parsed::Months(count), _) => Ok(Bucketing::Months { count, offset }),
    }
}

fn local(unit: LocalUnit, tz: chrono_tz::Tz) -> Bucketing {
    // Calendar boundaries sit on whole hours in every zone with a whole-hour offset (DST shifts
    // are whole hours too); the handful of :30/:45 zones need quarter-hour rows.
    let zone = Zone::Named(tz);
    let whole_hours = |at: DateTime<Utc>| zone.offset_millis(at) % HOUR == 0;
    let granularity = match whole_hours(Utc::now()) && whole_hours(Utc::now() - Duration::days(182))
    {
        true => HOUR,
        false => HOUR / 4,
    };

    Bucketing::Local {
        unit,
        tz,
        granularity,
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
        "h" => HOUR,
        "d" => DAY,
        _ => {
            return Err(Error::BadRequest(format!(
                "invalid fixed_interval [{value}]"
            )))
        }
    };

    match n.checked_mul(millis).filter(|m| *m > 0) {
        Some(millis) => Ok(millis),
        None => Err(Error::BadRequest(format!(
            "invalid fixed_interval [{value}]"
        ))),
    }
}

// ES spells these either as a word (`month`) or as `1`-prefixed shorthand (`1M`); multiples other
// than 1 are rejected, as in ES.
fn calendar_interval(value: &str) -> Result<Parsed, Error> {
    match value {
        "second" | "1s" => Ok(Parsed::Fixed(1_000)),
        "minute" | "1m" => Ok(Parsed::Fixed(60 * 1_000)),
        "hour" | "1h" => Ok(Parsed::Fixed(HOUR)),
        "day" | "1d" => Ok(Parsed::Day),
        "week" | "1w" => Ok(Parsed::Week),
        "month" | "1M" => Ok(Parsed::Months(1)),
        "quarter" | "1q" => Ok(Parsed::Months(3)),
        "year" | "1y" => Ok(Parsed::Months(12)),
        _ => Err(Error::BadRequest(format!(
            "invalid calendar_interval [{value}]"
        ))),
    }
}

impl Bucketing {
    // The grouping expression whose value identifies a document's engine row.
    pub fn key_expr(&self, name: &str) -> LogicalExpr {
        let at = |offset: i64| match offset {
            0 => field(name),
            offset => field(name).add(LogicalExpr::literal(offset)),
        };

        match self {
            Self::Shifted { width, offset } => at(*offset).div(LogicalExpr::literal(*width)),
            // Months elapsed since year 0, divided by the bucket's width in months. Folding the
            // year in is what keeps January 2025 and January 2026 in different buckets.
            Self::Months { count, offset } => at(*offset)
                .date_part("year")
                .mul(LogicalExpr::literal(12))
                .add(at(*offset).date_part("month"))
                .sub(LogicalExpr::literal(1))
                .div(LogicalExpr::literal(*count)),
            Self::Local { granularity, .. } => field(name).div(LogicalExpr::literal(*granularity)),
        }
    }

    // Engine row key -> start of the bucket the row belongs to, in UTC millis. For `Local` this
    // is many-to-one: every row inside one local day/week/month folds onto the same start.
    pub fn start_of_index(&self, index: i64) -> Option<i64> {
        match self {
            Self::Shifted { width, offset } => index.checked_mul(*width)?.checked_sub(*offset),
            Self::Months { count, offset } => {
                month_start(index.checked_mul(*count)?)?.checked_sub(*offset)
            }
            Self::Local { granularity, .. } => self.floor(index.checked_mul(*granularity)?),
        }
    }

    // Start of the bucket containing instant `t`.
    pub fn floor(&self, t: i64) -> Option<i64> {
        match self {
            Self::Shifted { width, offset } => {
                Some(t.checked_add(*offset)?.div_euclid(*width) * *width - *offset)
            }
            Self::Months { count, offset } => {
                let at = DateTime::<Utc>::from_timestamp_millis(t.checked_add(*offset)?)?;
                let months =
                    (at.year() as i64 * 12 + at.month0() as i64).div_euclid(*count) * *count;
                month_start(months)?.checked_sub(*offset)
            }
            Self::Local { unit, tz, .. } => {
                let date = DateTime::<Utc>::from_timestamp_millis(t)?
                    .with_timezone(tz)
                    .date_naive();
                let start = match unit {
                    LocalUnit::Day => date,
                    LocalUnit::Week => date.week(Weekday::Mon).first_day(),
                    LocalUnit::Months(n) => {
                        let months =
                            (date.year() as i64 * 12 + date.month0() as i64).div_euclid(*n) * *n;
                        NaiveDate::from_ymd_opt(
                            months.div_euclid(12) as i32,
                            (months.rem_euclid(12) + 1) as u32,
                            1,
                        )?
                    }
                };
                local_midnight(*tz, start)
            }
        }
    }

    // Start of the bucket after the one starting at `start`; drives empty-bucket filling.
    pub fn next(&self, start: i64) -> Option<i64> {
        match self {
            Self::Shifted { width, .. } => start.checked_add(*width),
            Self::Months { count, offset } => {
                let at = DateTime::<Utc>::from_timestamp_millis(start.checked_add(*offset)?)?;
                let months = at.year() as i64 * 12 + at.month0() as i64 + count;
                month_start(months)?.checked_sub(*offset)
            }
            Self::Local { unit, tz, .. } => {
                let date = DateTime::<Utc>::from_timestamp_millis(start)?
                    .with_timezone(tz)
                    .date_naive();
                let next = match unit {
                    LocalUnit::Day => date.checked_add_days(Days::new(1))?,
                    LocalUnit::Week => date.checked_add_days(Days::new(7))?,
                    LocalUnit::Months(n) => {
                        let months = date.year() as i64 * 12 + date.month0() as i64 + n;
                        NaiveDate::from_ymd_opt(
                            months.div_euclid(12) as i32,
                            (months.rem_euclid(12) + 1) as u32,
                            1,
                        )?
                    }
                };
                local_midnight(*tz, next)
            }
        }
    }
}

// First instant of `months` (counted from year 0) as UTC millis.
fn month_start(months: i64) -> Option<i64> {
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

// DST can skip or double local midnight; ES anchors such buckets at the earliest instant that
// exists on that date.
fn local_midnight(tz: chrono_tz::Tz, date: NaiveDate) -> Option<i64> {
    let naive = date.and_hms_opt(0, 0, 0)?;
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(dt) => Some(dt),
        LocalResult::Ambiguous(earliest, _) => Some(earliest),
        LocalResult::None => tz
            .from_local_datetime(&(naive + Duration::hours(1)))
            .earliest(),
    }
    .map(|dt| dt.timestamp_millis())
}
