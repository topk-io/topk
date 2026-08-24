use chrono::{DateTime, Datelike, Days, Duration, LocalResult, NaiveDate, TimeZone, Utc, Weekday};
use topk_rs::proto::v1::data::LogicalExpr;
use topk_rs::query::field;

use super::{Error, Zone};
use crate::api::DateHistogramBody;

const HOUR: i64 = 60 * 60 * 1_000;
const DAY: i64 = 24 * HOUR;

// How a date_histogram maps documents to buckets.
pub struct Bucketing {
    kind: Kind,

    // The `offset` request param: every boundary moves by this after the zone applies. Bucket
    // boundaries are computed on `t - shift` and shifted back, which is exact even for
    // variable-length calendar units.
    shift: i64,
}

// `Fixed`/`Months` bucket entirely in the engine on `ts + offset`, one engine row per bucket. A
// named `time_zone` observes DST, so no single offset is correct for a whole query: the `Local`
// kinds have the engine bucket by a granularity finer than any zone transition, then merge those
// rows into buckets here, where chrono-tz can follow the zone.
enum Kind {
    Fixed {
        width: i64,
        offset: i64,
    },
    Months {
        count: i64,
        offset: i64,
    },
    // Aligned to the zone-local epoch using the offset in force at the bucket's own instant,
    // which is what makes February buckets align to EST and July to EDT.
    LocalFixed {
        width: i64,
        tz: chrono_tz::Tz,
        granularity: i64,
    },
    LocalCalendar {
        unit: CalendarUnit,
        tz: chrono_tz::Tz,
        granularity: i64,
    },
}

#[derive(Clone, Copy)]
enum CalendarUnit {
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
    let shift = offset_param(h.offset.as_deref())?;

    // The epoch is a Thursday; pre-shifting by 3 days lands week buckets on Mondays, as ES does.
    const WEEK_SHIFT: i64 = 3 * DAY;

    // Under a fixed offset the zone is a constant, so every kind can bucket in the engine.
    let offset = match &zone {
        None => 0,
        Some(zone) => zone.offset_millis(Utc::now()),
    };

    let kind = match (parsed, zone) {
        (Parsed::Fixed(width), Some(Zone::Named(tz))) => local_fixed(width, tz, shift),
        (Parsed::Day, Some(Zone::Named(tz))) => local(CalendarUnit::Day, tz, shift),
        (Parsed::Week, Some(Zone::Named(tz))) => local(CalendarUnit::Week, tz, shift),
        (Parsed::Months(n), Some(Zone::Named(tz))) => local(CalendarUnit::Months(n), tz, shift),
        (Parsed::Fixed(width), _) => Kind::Fixed { width, offset },
        (Parsed::Day, _) => Kind::Fixed { width: DAY, offset },
        (Parsed::Week, _) => Kind::Fixed {
            width: 7 * DAY,
            offset: offset + WEEK_SHIFT,
        },
        (Parsed::Months(count), _) => Kind::Months { count, offset },
    };

    Ok(Bucketing { kind, shift })
}

// `+6h` / `-1d`: a signed fixed duration added to every bucket boundary.
fn offset_param(offset: Option<&str>) -> Result<i64, Error> {
    let Some(offset) = offset else { return Ok(0) };
    let (sign, duration) = match offset.strip_prefix('-') {
        Some(d) => (-1, d),
        None => (1, offset.strip_prefix('+').unwrap_or(offset)),
    };
    let millis = fixed_interval(duration)
        .map_err(|_| Error::BadRequest(format!("invalid offset [{offset}]")))?;
    Ok(sign * millis)
}

fn local(unit: CalendarUnit, tz: chrono_tz::Tz, shift: i64) -> Kind {
    Kind::LocalCalendar {
        unit,
        tz,
        granularity: granularity(tz, shift),
    }
}

fn local_fixed(width: i64, tz: chrono_tz::Tz, shift: i64) -> Kind {
    Kind::LocalFixed {
        width,
        tz,
        // A row has to nest inside the bucket it merges into, so it must divide the width too.
        granularity: gcd(granularity(tz, shift), width),
    }
}

// The engine row width for a locally bucketed zone. Calendar boundaries sit on whole hours in
// every zone with a whole-hour offset (DST shifts are whole hours too); the handful of :30/:45
// zones need quarter-hour rows. A boundary `shift` has to divide the row width as well.
fn granularity(tz: chrono_tz::Tz, shift: i64) -> i64 {
    let zone = Zone::Named(tz);
    let whole_hours = |at: DateTime<Utc>| zone.offset_millis(at) % HOUR == 0;
    let hours = whole_hours(Utc::now()) && whole_hours(Utc::now() - Duration::days(182));

    let granularity = match hours {
        true => HOUR,
        false => HOUR / 4,
    };
    match shift {
        0 => granularity,
        shift => gcd(granularity, shift.abs()),
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    match b {
        0 => a,
        b => gcd(b, a % b),
    }
}

// `<n><unit>` where unit is ms/s/m/h/d — ES rejects calendar units (w/M/y) here because their
// length varies.
fn fixed_interval(value: &str) -> Result<i64, Error> {
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
        // The engine buckets on the shifted field, so the boundary shift folds into the offset.
        let at = |offset: i64| match offset - self.shift {
            0 => field(name),
            offset => field(name).add(LogicalExpr::literal(offset)),
        };

        match &self.kind {
            Kind::Fixed { width, offset } => at(*offset).div(LogicalExpr::literal(*width)),
            // Months elapsed since year 0, divided by the bucket's width in months. Folding the
            // year in is what keeps January 2025 and January 2026 in different buckets.
            Kind::Months { count, offset } => at(*offset)
                .date_part("year")
                .mul(LogicalExpr::literal(12))
                .add(at(*offset).date_part("month"))
                .sub(LogicalExpr::literal(1))
                .div(LogicalExpr::literal(*count)),
            Kind::LocalFixed { granularity, .. } | Kind::LocalCalendar { granularity, .. } => {
                field(name).div(LogicalExpr::literal(*granularity))
            }
        }
    }

    // Engine row key -> start of the bucket the row belongs to, in UTC millis. For the `Local`
    // kinds this is many-to-one: every row inside one bucket folds onto the same start.
    pub fn start_of_index(&self, index: i64) -> Option<i64> {
        let start = match &self.kind {
            Kind::Fixed { width, offset } => index.checked_mul(*width)?.checked_sub(*offset)?,
            Kind::Months { count, offset } => {
                month_start(index.checked_mul(*count)?)?.checked_sub(*offset)?
            }
            Kind::LocalFixed { granularity, .. } | Kind::LocalCalendar { granularity, .. } => {
                return self.floor(index.checked_mul(*granularity)?)
            }
        };
        start.checked_add(self.shift)
    }

    // Start of the bucket containing instant `t`.
    pub fn floor(&self, t: i64) -> Option<i64> {
        self.floor_kind(t.checked_sub(self.shift)?)?
            .checked_add(self.shift)
    }

    // As `floor`, ignoring the boundary `offset`.
    fn floor_kind(&self, t: i64) -> Option<i64> {
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
            Kind::LocalFixed { width, tz, .. } => local_floor(*width, *tz, t)?,
            Kind::LocalCalendar { unit, tz, .. } => {
                let date = DateTime::<Utc>::from_timestamp_millis(t)?
                    .with_timezone(tz)
                    .date_naive();
                let start = match unit {
                    CalendarUnit::Day => date,
                    CalendarUnit::Week => date.week(Weekday::Mon).first_day(),
                    CalendarUnit::Months(n) => month_of(date, |m| m.div_euclid(*n) * *n)?,
                };
                from_local(*tz, midnight(start)?)?
            }
        })
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
            // A width step lands on the next boundary in absolute time, which is right inside a
            // repeated local hour. Across a transition the local grid moves instead, so the step
            // can land back inside this bucket; creep forward by the row width until the
            // rounding advances. A DST shift is at most a couple of hours, so this is a few
            // iterations at worst.
            Kind::LocalFixed {
                width,
                tz,
                granularity,
            } => {
                let mut t = start.checked_add(*width)?;
                loop {
                    let floored = local_floor(*width, *tz, t)?;
                    if floored > start {
                        break floored;
                    }
                    t = t.checked_add(*granularity)?;
                }
            }
            Kind::LocalCalendar { unit, tz, .. } => {
                let date = DateTime::<Utc>::from_timestamp_millis(start)?
                    .with_timezone(tz)
                    .date_naive();
                let next = match unit {
                    CalendarUnit::Day => date.checked_add_days(Days::new(1))?,
                    CalendarUnit::Week => date.checked_add_days(Days::new(7))?,
                    CalendarUnit::Months(n) => month_of(date, |m| m + n)?,
                };
                from_local(*tz, midnight(next)?)?
            }
        };
        next.checked_add(self.shift)
    }

    // As `floor`, but rounding in unshifted bucket space before `offset` moves the boundary —
    // which is how ES resolves `extended_bounds`.
    pub fn floor_unshifted(&self, t: i64) -> Option<i64> {
        self.floor(t.checked_add(self.shift)?)
    }

    // The `hard_bounds` window edge for `t`. ES rounds these ignoring `offset`, so the window is
    // not itself bucket-aligned once a boundary shift is in play — verified against
    // Elasticsearch 9 across positive and negative offsets, with and without a named zone.
    pub fn hard_bound(&self, t: i64) -> Option<i64> {
        self.floor_kind(t)
    }
}

// First instant of `months` (counted from year 0) as UTC millis.
fn month_start(months: i64) -> Option<i64> {
    let date = NaiveDate::from_ymd_opt(
        months.div_euclid(12) as i32,
        (months.rem_euclid(12) + 1) as u32,
        1,
    )?;
    midnight(date)
}

// The first of the month `f` maps `date`'s month to, counting months from year 0.
fn month_of(date: NaiveDate, f: impl Fn(i64) -> i64) -> Option<NaiveDate> {
    let months = f(date.year() as i64 * 12 + date.month0() as i64);
    NaiveDate::from_ymd_opt(
        months.div_euclid(12) as i32,
        (months.rem_euclid(12) + 1) as u32,
        1,
    )
}

fn midnight(date: NaiveDate) -> Option<i64> {
    Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis())
}

// Start of the `width`-wide bucket holding `t`, rounded in local wall-clock time so a boundary
// keeps the same local time across a DST transition even though the gap in absolute time is then
// shorter or longer.
fn local_floor(width: i64, tz: chrono_tz::Tz, t: i64) -> Option<i64> {
    let offset = offset_at(tz, t)?;
    let floored = t
        .checked_add(offset)?
        .div_euclid(width)
        .checked_mul(width)?;

    // Converted back at the instant's own offset, so the two halves of a repeated local hour
    // stay in separate buckets, as ES reports them.
    match floored.checked_sub(offset)? {
        back if offset_at(tz, back) == Some(offset) => Some(back),
        // The rounded local time does not exist at that offset (a skipped hour).
        _ => from_local(tz, floored),
    }
}

// The zone's UTC offset in force at `t`.
fn offset_at(tz: chrono_tz::Tz, t: i64) -> Option<i64> {
    let at = DateTime::<Utc>::from_timestamp_millis(t)?;
    Some(Zone::Named(tz).offset_millis(at))
}

// DST can skip or repeat a local time; ES anchors such a boundary at the earliest instant that
// exists, so a skipped midnight becomes 01:00 and a repeated hour its first occurrence.
fn from_local(tz: chrono_tz::Tz, local: i64) -> Option<i64> {
    let naive = DateTime::<Utc>::from_timestamp_millis(local)?.naive_utc();
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(dt) => Some(dt),
        LocalResult::Ambiguous(earliest, _) => Some(earliest),
        LocalResult::None => tz
            .from_local_datetime(&(naive + Duration::hours(1)))
            .earliest(),
    }
    .map(|dt| dt.timestamp_millis())
}
