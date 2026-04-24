use std::{
    io::{self, IsTerminal, Read},
    time::Duration,
};

use chrono::{DateTime, Local, Utc};

pub mod files;
pub mod mime;
pub mod progress;

pub use mime::MimeType;
use topk_rs::Error;

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
pub fn resolve_query(arg: Option<String>) -> Result<String, Error> {
    if let Some(q) = arg {
        return Ok(q);
    }

    if io::stdin().is_terminal() {
        return Err(Error::Input(anyhow::anyhow!(
            "query is required; pass it as an argument or pipe it via stdin"
        )));
    }

    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;

    Ok(buf.trim().to_string())
}
