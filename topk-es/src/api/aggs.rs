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

    // Zone-less date bounds are interpreted in this zone, as in a range query.
    #[serde(default)]
    pub time_zone: Option<String>,
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

    // Buckets are aligned to this zone rather than UTC — a numeric offset or an IANA name; see
    // `date::Bucketing` for how named zones follow DST.
    #[serde(default)]
    pub time_zone: Option<String>,

    // With `min_doc_count: 0` the histogram is filled out to cover at least [min, max], even
    // where no documents exist. Values are epoch millis or date-math strings.
    #[serde(default)]
    pub extended_bounds: Option<ExtendedBounds>,

    // The opposite of `extended_bounds`: buckets outside [min, max] are dropped. ES rejects
    // combining the two.
    #[serde(default)]
    pub hard_bounds: Option<ExtendedBounds>,

    // Shifts every bucket boundary by a fixed duration after the zone applies, e.g. `+6h` day
    // buckets running 06:00 to 06:00.
    #[serde(default)]
    pub offset: Option<String>,

    // `{"_key": "desc"}` or `{"_count": "asc"}`; ES also accepts sub-agg names, which we reject.
    #[serde(default)]
    pub order: Option<HashMap<String, String>>,

    // Keys are always epoch millis with an ISO companion; a key format pattern is accepted but
    // not interpreted, like mapping `format`.
    #[serde(default)]
    #[allow(dead_code)]
    pub format: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtendedBounds {
    #[serde(default)]
    pub min: Option<Value>,

    #[serde(default)]
    pub max: Option<Value>,
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
