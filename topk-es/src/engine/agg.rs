use std::collections::HashMap;

use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::data::{AggregateExpr, Document, LogicalExpr, Query as TopkQuery, Value};
use topk_rs::query::{field, filter};

use crate::api::{
    interval_millis, AggClause, AggResult, AggType, Bucket, TermsBucket, TopHitsBody,
};
use crate::value::{compare, ValueExt};
use crate::Error;

pub fn compile(clause: &AggClause, gate: &LogicalExpr) -> Result<TopkQuery, Error> {
    match &clause.ty {
        AggType::Terms(terms) => {
            let mut aggs = vec![("doc_count".to_string(), AggregateExpr::count(None))];
            for (name, sub_clause) in clause.aggs.iter().flatten() {
                aggs.push((
                    name.clone(),
                    AggregateExpr::try_from(sub_clause.ty.clone())?,
                ));
            }
            let query = filter(gate.clone())
                .group_by([("key".to_string(), field(terms.field.as_str()))], aggs)
                .sort("doc_count")
                .limit(terms.size.unwrap_or(10) as u64);

            Ok(query)
        }
        // date_histogram buckets on a computed key instead of a stored field: `ts / interval` is
        // the bucket index (ES aligns fixed_interval buckets to the epoch, not the data's own min,
        // so no separate "now"/origin lookup is needed), which is a single group_by like `terms`.
        // Sub-buckets still get filled with zero docs by ES (`min_doc_count: 0`); we only emit
        // buckets that actually have data, so a sparse series has fewer x-axis points than real ES.
        AggType::DateHistogram(h) => {
            let interval = interval_millis(h.fixed_interval.as_deref().ok_or_else(|| {
                Error::Unsupported(
                    "date_histogram requires \"fixed_interval\" (\"calendar_interval\" is not supported)".into(),
                )
            })?)?;
            let mut aggs = vec![("doc_count".to_string(), AggregateExpr::count(None))];
            for (name, sub_clause) in clause.aggs.iter().flatten() {
                aggs.push((
                    name.clone(),
                    AggregateExpr::try_from(sub_clause.ty.clone())?,
                ));
            }
            let key = field(h.field.as_str()).div(LogicalExpr::literal(interval));
            Ok(filter(gate.clone())
                .group_by([("key".to_string(), key)], aggs)
                .sort("key")
                // ES date_histogram has no inherent bucket cap; this is generous headroom, not
                // an ES-shaped limit.
                .limit(10_000))
        }
        // filter/missing/range: sub-bucket aggregations need per-bucket queries the one-query-per-
        // agg model can't express, so we return the outer gate's count and shape buckets in
        // collect(). Real per-bucket counts for range are a follow-up (see ELASTIC.md).
        AggType::Filter(_) | AggType::Missing(_) | AggType::Range(_) => {
            Ok(filter(gate.clone()).count())
        }
        AggType::TopHits => Ok(filter(gate.clone()).count()),
        metric => {
            let query = filter(gate.clone()).group_by(
                [("_bucket".to_string(), LogicalExpr::literal(true))],
                [(
                    "value".to_string(),
                    AggregateExpr::try_from(metric.clone())?,
                )],
            );
            Ok(query)
        }
    }
}

pub fn collect(clause: &AggClause, docs: Vec<Document>) -> Result<AggResult, Error> {
    match &clause.ty {
        AggType::Terms(_) => {
            let mut buckets = Vec::with_capacity(docs.len());

            for mut doc in docs {
                let raw = doc.fields.remove("key").unwrap_or_else(Value::null);
                // ES reports boolean terms keys as 1/0 with a "true"/"false" companion.
                let (key, key_as_string) = match raw.as_bool() {
                    Some(b) => (JsonValue::from(Value::i64(b as i64)), Some(b.to_string())),
                    None => (JsonValue::from(raw), None),
                };

                let doc_count = doc
                    .fields
                    .remove("doc_count")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);

                let mut sub_aggs = HashMap::new();
                for (name, _) in clause.aggs.iter().flatten() {
                    let value = doc.fields.remove(name).and_then(|v| v.number());
                    sub_aggs.insert(name.clone(), AggResult::Metric { value });
                }

                buckets.push(TermsBucket {
                    key,
                    key_as_string,
                    doc_count,
                    sub_aggs,
                });
            }

            // ES breaks `doc_count` ties by key, ascending.
            buckets.sort_by(|a, b| {
                b.doc_count
                    .cmp(&a.doc_count)
                    .then_with(|| compare(&a.key, &b.key))
            });

            Ok(AggResult::Terms {
                doc_count_error_upper_bound: 0,
                sum_other_doc_count: 0,
                buckets,
            })
        }
        AggType::TopHits => Ok(AggResult::TopHits {
            hits: TopHitsBody::default(),
        }),
        AggType::Filter(_) | AggType::Missing(_) => {
            let doc_count = docs
                .into_iter()
                .next()
                .and_then(|mut doc| {
                    doc.fields
                        .remove("_count")
                        .or_else(|| doc.fields.remove("count"))
                })
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            Ok(AggResult::Single {
                doc_count,
                sub_aggs: HashMap::new(),
            })
        }
        AggType::Range(r) => Ok(AggResult::Buckets {
            buckets: r
                .ranges
                .iter()
                .map(|_| Bucket {
                    key: None,
                    key_as_string: None,
                    from: None,
                    to: None,
                    doc_count: 0,
                    sub_aggs: HashMap::new(),
                })
                .collect(),
        }),
        AggType::DateHistogram(h) => {
            let interval = interval_millis(h.fixed_interval.as_deref().unwrap_or("1s"))?;
            let mut buckets = Vec::with_capacity(docs.len());

            for mut doc in docs {
                let index = doc
                    .fields
                    .remove("key")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let millis = index * interval;

                let doc_count = doc
                    .fields
                    .remove("doc_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let mut sub_aggs = HashMap::new();
                for (name, _) in clause.aggs.iter().flatten() {
                    let value = doc.fields.remove(name).and_then(|v| v.number());
                    sub_aggs.insert(name.clone(), AggResult::Metric { value });
                }

                buckets.push(Bucket {
                    key: Some(serde_json::json!(millis)),
                    key_as_string: crate::date::format_millis(millis),
                    from: None,
                    to: None,
                    doc_count,
                    sub_aggs,
                });
            }

            buckets.sort_by_key(|b| b.key.as_ref().and_then(|k| k.as_i64()).unwrap_or(0));
            Ok(AggResult::Buckets { buckets })
        }
        _ => {
            let value = docs
                .into_iter()
                .next()
                .and_then(|mut doc| doc.fields.remove("value"))
                .and_then(|v| v.number());

            // Over an empty match set ES sums and counts to 0; avg/min/max stay null.
            let value = match (value, &clause.ty) {
                (None, AggType::Sum(_) | AggType::ValueCount(_)) => Some(0.0),
                (value, _) => value,
            };

            Ok(AggResult::Metric { value })
        }
    }
}

impl TryFrom<AggType> for AggregateExpr {
    type Error = Error;
    fn try_from(value: AggType) -> Result<Self, Self::Error> {
        match value {
            AggType::Sum(m) => Ok(AggregateExpr::sum(m.field)),
            AggType::Avg(m) => Ok(AggregateExpr::avg(m.field)),
            AggType::Min(m) => Ok(AggregateExpr::min(m.field)),
            AggType::Max(m) => Ok(AggregateExpr::max(m.field)),
            AggType::ValueCount(m) => Ok(AggregateExpr::count(Some(m.field.into()))),
            other => Err(Error::Unsupported(format!(
                "aggregation {} is not a metric sub-aggregation",
                other.name()
            ))),
        }
    }
}
