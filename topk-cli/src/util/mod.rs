use std::{
    io::{self, IsTerminal, Read},
    time::Duration,
};

use anyhow::Result;
use chrono::{DateTime, Local, Utc};

pub mod files;
pub mod mime;
pub mod progress;

pub use mime::MimeType;

pub fn format_timestamp(rfc3339: &str) -> Option<String> {
    rfc3339.parse::<DateTime<Utc>>().ok().map(|dt| {
        dt.with_timezone(&Local)
            .format("%b %-d, %Y %H:%M")
            .to_string()
    })
}

pub fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

pub fn parse_seconds(value: &str) -> Result<Duration, String> {
    value
        .parse::<u64>()
        .map(Duration::from_secs)
        .map_err(|err| err.to_string())
}

/// Resolves a query string from an optional CLI argument, falling back to stdin
pub fn resolve_query(arg: Option<String>) -> Result<Option<String>> {
    if let Some(q) = arg {
        return Ok(Some(q));
    }

    if io::stdin().is_terminal() {
        return Ok(None);
    }

    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;

    let q = buf.trim().to_string();

    if q.is_empty() {
        Ok(None)
    } else {
        Ok(Some(q))
    }
}
