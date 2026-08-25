use std::collections::HashMap;

use chrono::Utc;

use serde::{Deserialize, Deserializer, Serialize};
use topk_rs::json::Value;

use super::query::FieldName;
use crate::date::{self, Bucketing, Grid, Zone};
use crate::Error;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggClause {
    #[serde(flatten)]
    pub ty: AggType,

    #[serde(default, alias = "aggregations")]
    pub aggs: Option<HashMap<String, AggClause>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggType {
    Terms(TermsAggBody),
    Sum(MetricAggBody),
    Avg(MetricAggBody),
    Min(MetricAggBody),
    Max(MetricAggBody),
    ValueCount(MetricAggBody),
    DateHistogram(DateHistogramBody),
    Range(RangeAggBody),
    DateRange(RangeAggBody),
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TermsAggBody {
    pub field: FieldName,

    #[serde(default)]
    pub size: Option<u32>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeAggBody {
    pub field: FieldName,
    pub ranges: Vec<RangeSpec>,

    // Zone-less date bounds are interpreted in this zone, as in a range query.
    #[serde(default)]
    pub time_zone: Option<Zone>,
}

// `from` is inclusive and `to` exclusive, as in ES; omitting one leaves that side unbounded.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeSpec {
    #[serde(default)]
    pub key: Option<String>,

    #[serde(default)]
    pub from: Option<Value>,

    #[serde(default)]
    pub to: Option<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DateHistogramRaw {
    field: FieldName,

    #[serde(default)]
    fixed_interval: Option<String>,

    #[serde(default)]
    calendar_interval: Option<String>,

    #[serde(default)]
    min_doc_count: Option<u64>,

    #[serde(default)]
    time_zone: Option<Zone>,

    #[serde(default)]
    extended_bounds: Option<Bounds>,

    #[serde(default)]
    hard_bounds: Option<Bounds>,

    #[serde(default)]
    offset: Option<String>,

    #[serde(default)]
    order: Option<Order>,

    #[serde(default, rename = "format")]
    _format: Option<String>,
}

#[derive(Clone)]
pub struct DateHistogramBody {
    pub field: FieldName,
    pub bucketing: Bucketing,
    pub zone: Option<Zone>,

    pub extended_bounds: Option<Bounds>,

    pub hard_bounds: Option<Bounds>,

    pub min_doc_count: u64,
    pub order: Option<Order>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    #[serde(default)]
    pub min: Option<Value>,

    #[serde(default)]
    pub max: Option<Value>,
}

impl TryFrom<DateHistogramRaw> for DateHistogramBody {
    type Error = Error;

    fn try_from(raw: DateHistogramRaw) -> Result<Self, Error> {
        let (grid, epoch) =
            match (raw.fixed_interval, raw.calendar_interval) {
                (Some(_), Some(_)) => return Err(Error::BadRequest(
                    "date_histogram accepts either fixed_interval or calendar_interval, not both"
                        .into(),
                )),
                (Some(fixed), None) => (Grid::Fixed(date::parse_fixed_interval(&fixed)?), 0),
                (None, Some(calendar)) => date::parse_calendar_interval(&calendar)?,
                (None, None) => {
                    return Err(Error::BadRequest(
                        "date_histogram requires fixed_interval or calendar_interval".into(),
                    ))
                }
            };

        let offset = raw
            .time_zone
            .map(|zone| zone.offset_millis(Utc::now()))
            .unwrap_or(0);
        let shift = raw
            .offset
            .as_deref()
            .map(date::parse_offset)
            .transpose()?
            .unwrap_or(0);

        Ok(Self {
            field: raw.field,
            bucketing: Bucketing::new(grid, offset + epoch, shift),
            zone: raw.time_zone,
            extended_bounds: raw.extended_bounds,
            hard_bounds: raw.hard_bounds,
            min_doc_count: raw.min_doc_count.unwrap_or(0),
            order: raw.order,
        })
    }
}

impl<'de> Deserialize<'de> for DateHistogramBody {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        DateHistogramRaw::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Deserialize)]
pub enum Order {
    #[serde(rename = "_key")]
    Key(Direction),

    #[serde(rename = "_count")]
    Count(Direction),
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Asc,
    Desc,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(json: &str) -> Result<Order, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn order_shapes() {
        assert!(matches!(
            order(r#"{"_key": "desc"}"#).unwrap(),
            Order::Key(Direction::Desc)
        ));
        assert!(matches!(
            order(r#"{"_count": "asc"}"#).unwrap(),
            Order::Count(Direction::Asc)
        ));
        assert!(order("{}").is_err());
        assert!(order(r#"{"_key": "sideways"}"#).is_err());
        assert!(order(r#"{"my_sub_agg": "asc"}"#).is_err());
        assert!(order(r#"{"_key": "desc", "_count": "asc"}"#).is_err());
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricAggBody {
    pub field: FieldName,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum AggResult {
    Metric {
        value: Option<f64>,

        #[serde(skip_serializing_if = "Option::is_none")]
        value_as_string: Option<String>,
    },
    Terms {
        doc_count_error_upper_bound: u32,
        sum_other_doc_count: u64,
        buckets: Vec<TermsBucket>,
    },
    Histogram {
        buckets: Vec<HistogramBucket>,
    },
    Range {
        buckets: Vec<RangeBucket>,
    },
}

#[derive(Serialize)]
pub struct TermsBucket {
    pub key: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_as_string: Option<String>,
    pub doc_count: u64,
    #[serde(flatten)]
    pub sub_aggs: HashMap<String, AggResult>,
}

#[derive(Serialize)]
pub struct HistogramBucket {
    pub key: i64,
    pub key_as_string: String,
    pub doc_count: u64,

    #[serde(flatten)]
    pub sub_aggs: HashMap<String, AggResult>,
}

#[derive(Serialize)]
pub struct RangeBucket {
    pub key: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_as_string: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_as_string: Option<String>,

    pub doc_count: u64,
}
