use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use topk_rs::json::Value;

use super::query::FieldName;
use crate::Error;

#[derive(Deserialize)]
pub struct AggClause {
    #[serde(flatten)]
    pub ty: AggType,

    #[serde(default, alias = "aggregations")]
    pub aggs: Option<HashMap<String, AggClause>>,
}

#[derive(Clone)]
pub enum AggType {
    Terms(TermsAggBody),
    Sum(MetricAggBody),
    Avg(MetricAggBody),
    Min(MetricAggBody),
    Max(MetricAggBody),
    ValueCount(MetricAggBody),
    Filter(serde_json::Value),
    Missing(MetricAggBody),
    Range(RangeAggBody),
    DateHistogram(DateHistogramBody),
    // STUB: accepted only as a sub-agg; always reports an empty hit set. See ELASTIC.md.
    TopHits,
}

impl AggType {
    pub fn name(&self) -> &'static str {
        match self {
            AggType::Terms(_) => "terms",
            AggType::Sum(_) => "sum",
            AggType::Avg(_) => "avg",
            AggType::Min(_) => "min",
            AggType::Max(_) => "max",
            AggType::ValueCount(_) => "value_count",
            AggType::Filter(_) => "filter",
            AggType::Missing(_) => "missing",
            AggType::Range(_) => "range",
            AggType::DateHistogram(_) => "date_histogram",
            AggType::TopHits => "top_hits",
        }
    }
}

// Hand-rolled so an unsupported aggregation names itself; the derived flattened-enum error says
// nothing to a caller. `aggs`/`aggregations` is AggClause's own field but reaches us through the
// flatten, so the agg type is the first key that is neither.
impl<'de> Deserialize<'de> for AggType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let clause = serde_json::Map::deserialize(deserializer)?;

        let (name, body) = match clause
            .into_iter()
            .find(|(key, _)| key != "aggs" && key != "aggregations")
        {
            Some(entry) => entry,
            None => return Err(serde::de::Error::custom("empty aggregation clause")),
        };

        fn parse<T: serde::de::DeserializeOwned, E: serde::de::Error>(
            body: serde_json::Value,
        ) -> Result<T, E> {
            serde_json::from_value(body).map_err(serde::de::Error::custom)
        }

        match name.as_str() {
            "terms" => parse(body).map(AggType::Terms),
            "sum" => parse(body).map(AggType::Sum),
            "avg" => parse(body).map(AggType::Avg),
            "min" => parse(body).map(AggType::Min),
            "max" => parse(body).map(AggType::Max),
            "value_count" => parse(body).map(AggType::ValueCount),
            "filter" => Ok(AggType::Filter(body)),
            "missing" => parse(body).map(AggType::Missing),
            "range" => parse(body).map(AggType::Range),
            "date_histogram" => parse(body).map(AggType::DateHistogram),
            "top_hits" => Ok(AggType::TopHits),
            other => Err(serde::de::Error::custom(format!(
                "unsupported aggregation `{other}`"
            ))),
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct TermsAggBody {
    pub field: FieldName,

    #[serde(default)]
    pub size: Option<u32>,

    // Docs missing the field are dropped rather than counted under this key; no missing bucket.
    #[serde(default)]
    #[allow(dead_code)]
    pub missing: Option<serde_json::Value>,
}

#[derive(Clone, Deserialize)]
pub struct MetricAggBody {
    pub field: FieldName,
}

#[derive(Clone, Deserialize)]
pub struct RangeAggBody {
    pub field: FieldName,
    pub ranges: Vec<RangeSpec>,
}

#[derive(Clone, Deserialize)]
pub struct RangeSpec {
    #[serde(default)]
    pub from: Option<serde_json::Value>,
    #[serde(default)]
    pub to: Option<serde_json::Value>,
}

#[derive(Clone, Deserialize)]
pub struct DateHistogramBody {
    pub field: FieldName,
    #[serde(default)]
    pub fixed_interval: Option<String>,
    #[serde(default)]
    pub calendar_interval: Option<String>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum AggResult {
    Metric {
        value: Option<f64>,
    },
    Buckets {
        buckets: Vec<Bucket>,
    },
    Terms {
        doc_count_error_upper_bound: u32,
        sum_other_doc_count: u64,
        buckets: Vec<TermsBucket>,
    },
    Single {
        doc_count: u64,
        #[serde(flatten)]
        sub_aggs: HashMap<String, AggResult>,
    },
    TopHits {
        hits: TopHitsBody,
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
pub struct Bucket {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_as_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<f64>,
    pub doc_count: u64,
    #[serde(flatten)]
    pub sub_aggs: HashMap<String, AggResult>,
}

#[derive(Serialize)]
pub struct TopHitsBody {
    pub total: TopHitsTotal,
    pub max_score: Option<f64>,
    pub hits: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct TopHitsTotal {
    pub value: u64,
    pub relation: &'static str,
}

impl Default for TopHitsBody {
    fn default() -> Self {
        TopHitsBody {
            total: TopHitsTotal {
                value: 0,
                relation: "eq",
            },
            max_score: None,
            hits: Vec::new(),
        }
    }
}

// A date_histogram/range interval as milliseconds. `fixed_interval` only; calendar units need
// date_part (available on the engine, not yet piped here) and nothing on the boot path uses them.
pub fn interval_millis(spec: &str) -> Result<i64, Error> {
    let (num, unit) = spec.split_at(
        spec.find(|c: char| !c.is_ascii_digit())
            .unwrap_or(spec.len()),
    );
    let num: i64 = num
        .parse()
        .map_err(|_| Error::BadRequest(format!("invalid interval [{spec}]")))?;
    let unit_ms = match unit {
        "ms" => 1,
        "s" => 1000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => {
            return Err(Error::Unsupported(format!(
                "unsupported histogram interval [{spec}]; only fixed ms/s/m/h/d"
            )))
        }
    };
    Ok(num * unit_ms)
}
