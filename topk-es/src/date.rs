use chrono::{DateTime, SecondsFormat, Utc};

use crate::Error;

// ES `date` values arrive as ISO-8601 strings (or already-epoch millis) and must land in TopK's
// timestamp column as i64 millis. Reads go the other way. Parsing/formatting is pure; date math
// (`now`, `now-30s`) needs the wall clock and lives in the proxy, not here.
pub fn parse_millis(value: &str) -> Result<i64, Error> {
    if let Ok(millis) = value.parse::<i64>() {
        return Ok(millis);
    }

    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.timestamp_millis())
        .map_err(|_| Error::BadRequest(format!("cannot parse date [{value}]")))
}

pub fn format_millis(millis: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(millis)
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
}
