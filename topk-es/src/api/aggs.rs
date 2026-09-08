use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, Map};
use topk_rs::json::Value;

use super::query::FieldName;
use crate::date::Zone;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggClause {
    #[serde(flatten)]
    pub ty: AggType,

    #[serde(default, alias = "aggregations")]
    pub aggs: HashMap<String, AggClause>,
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
    Cardinality(CardinalityAggBody),
    DateHistogram(DateHistogramBody),
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardinalityAggBody {
    pub field: FieldName,

    // Accepted, not honoured: `count_distinct` exposes no accuracy dial to feed it.
    #[serde(default)]
    #[allow(dead_code)]
    pub precision_threshold: Option<u32>,
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
pub struct DateHistogramBody {
    pub field: FieldName,

    #[serde(default)]
    pub fixed_interval: Option<String>,

    #[serde(default)]
    pub calendar_interval: Option<String>,

    #[serde(default)]
    pub min_doc_count: Option<u64>,

    #[serde(default)]
    pub time_zone: Zone,

    #[serde(default)]
    pub extended_bounds: Option<Bounds>,

    #[serde(default)]
    pub keyed: bool,

    #[serde(default, rename = "format")]
    _format: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
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
        buckets: Vec<Bucket>,
    },
    Histogram {
        buckets: Buckets,
    },
}

#[serde_as]
#[derive(Serialize)]
#[serde(untagged)]
pub enum Buckets {
    List(Vec<Bucket>),
    Keyed(#[serde_as(as = "Map<_, _>")] Vec<(String, Bucket)>),
}

#[derive(Serialize)]
pub struct Bucket {
    pub key: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_as_string: Option<String>,
    pub doc_count: u64,
    #[serde(flatten)]
    pub sub_aggs: HashMap<String, AggResult>,
}
