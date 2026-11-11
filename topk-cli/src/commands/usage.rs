use std::io::Write;
use std::process::ExitCode;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use bytesize::ByteSize;
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, SecondsFormat, Utc};
use serde::{Serialize, Serializer};

use crate::client::ManagementClient;
use crate::config::Config;
use crate::management::proto::{GetUsageRequest, UsageMetric, UsageUnit};
use crate::output::{print_list, Output, Tabular};
use crate::ProjectId;

#[derive(clap::Args)]
pub struct Args {
    /// Start of the range, at most 90 days before the end: a duration ago (<n>m|h|d|w), a date
    /// (YYYY-MM-DD) or an RFC 3339 time
    #[arg(long, value_name = "TIME")]
    pub since: Time,
    /// End of the range: a duration ago, a date or an RFC 3339 time; defaults to now
    #[arg(long, value_name = "TIME")]
    pub until: Option<Time>,
    /// Limit to this project; defaults to the selected project, or the whole organization if
    /// none is selected
    #[arg(long, env = "TOPK_PROJECT_ID")]
    pub project_id: Option<ProjectId>,
    /// Limit to this collection in the project
    #[arg(long, value_name = "NAME")]
    pub collection: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Time {
    /// This long before now
    Ago(Duration),
    /// A date is midnight UTC at its start
    At(DateTime<Utc>),
}

impl Time {
    fn resolve(self, now: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Time::Ago(duration) => now - duration,
            Time::At(time) => time,
        }
    }
}

impl FromStr for Time {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Ok(duration) = humantime::parse_duration(value) {
            return Duration::from_std(duration)
                .map(Time::Ago)
                .map_err(|_| format!("duration too long: {value:?}"));
        }
        DateTime::parse_from_rfc3339(value)
            .map(|time| time.to_utc())
            .or_else(|_| {
                NaiveDate::parse_from_str(value, "%Y-%m-%d")
                    .map(|date| date.and_time(NaiveTime::MIN).and_utc())
            })
            .map(Time::At)
            .map_err(|_| {
                format!(
                    "expected a duration (<n>m|h|d|w), a date (YYYY-MM-DD) or RFC 3339 \
                     (2026-11-01T00:00:00Z), got {value:?}"
                )
            })
    }
}

#[derive(Serialize)]
struct Row {
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<ProjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    collection: Option<String>,
    metric: String,
    #[serde(serialize_with = "serialize_unit")]
    unit: UsageUnit,
    value: u64,
}

fn serialize_unit<S: Serializer>(unit: &UsageUnit, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(match unit {
        UsageUnit::Count => "count",
        UsageUnit::Bytes => "bytes",
        UsageUnit::ByteHours => "byte_hours",
        // A unit this CLI doesn't know yet
        UsageUnit::Unspecified => "unspecified",
    })
}

impl Row {
    fn new(metric: UsageMetric, project_id: Option<ProjectId>, collection: Option<String>) -> Self {
        Self {
            project_id,
            collection,
            unit: metric.unit(),
            metric: metric.description,
            value: metric.value,
        }
    }
}

impl Tabular for Row {
    fn columns(&self) -> Vec<(&'static str, String)> {
        let value = match self.unit {
            UsageUnit::Bytes => ByteSize(self.value).to_string_as(false),
            UsageUnit::ByteHours => {
                format!("{}-hours", ByteSize(self.value).to_string_as(false))
            }
            UsageUnit::Count | UsageUnit::Unspecified => self.value.to_string(),
        };
        let project = self
            .project_id
            .as_ref()
            .map(|id| ("Project", id.to_string()));
        let collection = self.collection.clone().map(|name| ("Collection", name));
        project
            .into_iter()
            .chain(collection)
            .chain([("Metric", self.metric.clone()), ("Value", value)])
            .collect()
    }
}

pub async fn run(
    config: Config,
    args: &Args,
    output: Output,
    out: &mut impl Write,
) -> Result<ExitCode> {
    let project_id = match &args.project_id {
        Some(project_id) => Some(project_id.clone()),
        None => config.defaults()?.project_id,
    };
    if args.collection.is_some() && project_id.is_none() {
        bail!(
            "--collection needs a project: pass --project-id or select one with \
             `topk project select`"
        );
    }
    let now = Utc::now();
    let start_time = args.since.resolve(now);
    let end_time = args.until.map_or(now, |until| until.resolve(now));
    let metrics = ManagementClient::new(config)?
        .usage
        .get_usage(GetUsageRequest {
            start_time: start_time.to_rfc3339_opts(SecondsFormat::Secs, true),
            end_time: end_time.to_rfc3339_opts(SecondsFormat::Secs, true),
            project_id: project_id.as_ref().map(ToString::to_string),
            collection: args.collection.clone(),
        })
        .await
        .context("failed to get usage")?
        .into_inner()
        .metrics;
    print_list(
        out,
        output,
        metrics
            .into_iter()
            .map(|metric| Row::new(metric, project_id.clone(), args.collection.clone())),
    )?;
    Ok(ExitCode::SUCCESS)
}
