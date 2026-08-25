use chrono::{DateTime, Datelike, NaiveDate, Utc};
use topk_rs::proto::v1::data::LogicalExpr;
use topk_rs::query::field;

use super::Error;

const HOUR: i64 = 60 * 60 * 1_000;
const DAY: i64 = 24 * HOUR;

#[derive(Clone, Copy)]
pub struct Bucketing {
    grid: Grid,
    offset: i64,
    shift: i64,
}

#[derive(Clone, Copy)]
pub enum Grid {
    Fixed(i64),
    Months(i64),
}

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

pub fn parse_calendar_interval(value: &str) -> Result<(Grid, i64), Error> {
    match value {
        "second" | "1s" => Ok((Grid::Fixed(1_000), 0)),
        "minute" | "1m" => Ok((Grid::Fixed(60 * 1_000), 0)),
        "hour" | "1h" => Ok((Grid::Fixed(HOUR), 0)),
        "day" | "1d" => Ok((Grid::Fixed(DAY), 0)),
        "week" | "1w" => Ok((Grid::Fixed(7 * DAY), 3 * DAY)),
        "month" | "1M" => Ok((Grid::Months(1), 0)),
        "quarter" | "1q" => Ok((Grid::Months(3), 0)),
        "year" | "1y" => Ok((Grid::Months(12), 0)),
        _ => Err(Error::BadRequest(format!(
            "invalid calendar_interval [{value}]"
        ))),
    }
}

impl Bucketing {
    pub fn new(grid: Grid, offset: i64, shift: i64) -> Self {
        Self {
            grid,
            offset,
            shift,
        }
    }

    fn shifted(&self, name: &str) -> LogicalExpr {
        match self.offset - self.shift {
            0 => field(name),
            offset => field(name).add(LogicalExpr::literal(offset)),
        }
    }

    pub fn key_expr(&self, name: &str) -> LogicalExpr {
        match self.grid {
            Grid::Fixed(width) => self.shifted(name).div(LogicalExpr::literal(width)),
            Grid::Months(count) => self
                .shifted(name)
                .date_part("year")
                .mul(LogicalExpr::literal(12))
                .add(self.shifted(name).date_part("month"))
                .sub(LogicalExpr::literal(1))
                .div(LogicalExpr::literal(count)),
        }
    }

    pub fn start_of_index(&self, index: i64) -> Option<i64> {
        let start = match self.grid {
            Grid::Fixed(width) => index.checked_mul(width)?.checked_sub(self.offset)?,
            Grid::Months(count) => {
                month_start(index.checked_mul(count)?)?.checked_sub(self.offset)?
            }
        };
        start.checked_add(self.shift)
    }

    pub fn floor_unshifted(&self, t: i64) -> Option<i64> {
        let t = t.checked_add(self.offset)?;
        Some(match self.grid {
            Grid::Fixed(width) => t.div_euclid(width) * width - self.offset,
            Grid::Months(count) => {
                let at = DateTime::<Utc>::from_timestamp_millis(t)?;
                let months = (at.year() as i64 * 12 + at.month0() as i64).div_euclid(count) * count;
                month_start(months)?.checked_sub(self.offset)?
            }
        })
    }

    pub fn extended_bound(&self, t: i64) -> Option<i64> {
        self.floor_unshifted(t)?.checked_add(self.shift)
    }

    pub fn next(&self, start: i64) -> Option<i64> {
        let start = start.checked_sub(self.shift)?;
        let next = match self.grid {
            Grid::Fixed(width) => start.checked_add(width)?,
            Grid::Months(count) => {
                let at = DateTime::<Utc>::from_timestamp_millis(start.checked_add(self.offset)?)?;
                let months = at.year() as i64 * 12 + at.month0() as i64 + count;
                month_start(months)?.checked_sub(self.offset)?
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
