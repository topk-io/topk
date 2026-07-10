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
pub struct MetricAggBody {
    pub field: FieldName,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum AggResult {
    Metric {
        value: Option<f64>,
    },
    Terms {
        doc_count_error_upper_bound: u32,
        sum_other_doc_count: u64,
        buckets: Vec<TermsBucket>,
    },
}

#[derive(Serialize)]
pub struct TermsBucket {
    pub key: Value,
    pub doc_count: u64,
    #[serde(flatten)]
    pub sub_aggs: HashMap<String, AggResult>,
}
