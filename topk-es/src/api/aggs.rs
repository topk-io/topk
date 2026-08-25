use std::collections::HashMap;
use std::fmt;

use chrono::Utc;

use serde::{Deserialize, Deserializer, Serialize};
use topk_rs::json::Value;

use super::query::FieldName;
use crate::date::{self, Interval, Zone};
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
    pub interval: Interval,

    pub shift: i64,

    pub offset: i64,

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
        let interval =
            match (raw.fixed_interval, raw.calendar_interval) {
                (Some(_), Some(_)) => return Err(Error::BadRequest(
                    "date_histogram accepts either fixed_interval or calendar_interval, not both"
                        .into(),
                )),
                (Some(fixed), None) => Interval::Fixed(date::parse_fixed_interval(&fixed)?),
                (None, Some(calendar)) => date::parse_calendar_interval(&calendar)?,
                (None, None) => {
                    return Err(Error::BadRequest(
                        "date_histogram requires fixed_interval or calendar_interval".into(),
                    ))
                }
            };

        Ok(Self {
            field: raw.field,
            interval,
            shift: raw
                .offset
                .as_deref()
                .map(date::parse_offset)
                .transpose()?
                .unwrap_or(0),
            offset: raw
                .time_zone
                .map(|zone| zone.offset_millis(Utc::now()))
                .unwrap_or(0),
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

#[derive(Clone, Copy)]
pub struct Order {
    pub target: OrderTarget,
    pub direction: Direction,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum OrderTarget {
    #[serde(rename = "_key")]
    Key,

    #[serde(rename = "_count")]
    Count,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Asc,
    Desc,
}

impl<'de> Deserialize<'de> for Order {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct First;

        impl<'de> serde::de::Visitor<'de> for First {
            type Value = Order;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(r#"an object like {"_key": "desc"}"#)
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Order, A::Error> {
                let (target, direction) = map.next_entry()?.ok_or_else(|| {
                    serde::de::Error::custom("date_histogram order must name a target")
                })?;

                while map.next_entry::<OrderTarget, Direction>()?.is_some() {}

                Ok(Order { target, direction })
            }
        }

        deserializer.deserialize_map(First)
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
