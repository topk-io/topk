use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use topk_rs::json::Value;

use super::query::FieldName;

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

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DateHistogramBody {
    pub field: FieldName,

    // ES accepts either; `fixed_interval` is an exact duration, `calendar_interval` follows the
    // calendar (a month is not 30 days). Exactly one must be given.
    #[serde(default)]
    pub fixed_interval: Option<String>,

    #[serde(default)]
    pub calendar_interval: Option<String>,

    #[serde(default)]
    pub min_doc_count: Option<u64>,

    // Buckets are aligned to this zone rather than UTC. Only numeric offsets are accepted: the
    // engine cannot convert per document, so bucketing shifts by one constant, which a named
    // zone's DST transitions would silently break.
    #[serde(default)]
    pub time_zone: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

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
