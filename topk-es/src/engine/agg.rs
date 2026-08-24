use std::collections::HashMap;

use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::data::{AggregateExpr, Document, LogicalExpr, Query as TopkQuery, Value};
use topk_rs::query::{field, filter};

use super::{Schema, RANGE_PREFIX};
use topk_rs::proto::v1::control::FieldSpec;
use crate::api::{
    AggClause, AggResult, AggType, HistogramBucket, RangeAggBody, RangeBucket, RangeSpec,
    TermsBucket,
};
use crate::date;
use crate::value::{compare, ValueExt};
use crate::Error;

pub fn compile(
    schema: &Schema,
    clause: &AggClause,
    gate: &LogicalExpr,
) -> Result<TopkQuery, Error> {
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
        AggType::DateHistogram(h) => {
            let mut aggs = vec![("doc_count".to_string(), AggregateExpr::count(None))];
            for (name, sub_clause) in clause.aggs.iter().flatten() {
                aggs.push((
                    name.clone(),
                    AggregateExpr::try_from(sub_clause.ty.clone())?,
                ));
            }

            // Bucket by integer division: `ts / interval` is the bucket index, multiplied back to
            // a timestamp in `collect`. Calendar intervals are irregular, so those group by the
            // relevant date part instead.
            let key = match date::interval(h)? {
                date::Interval::Fixed(millis) => {
                    field(h.field.as_str()).div(LogicalExpr::literal(millis))
                }
                date::Interval::Year => field(h.field.as_str()).date_part("year"),
                // Month numbers repeat every year, so fold the year in to keep buckets distinct.
                date::Interval::Month => field(h.field.as_str())
                    .date_part("year")
                    .mul(LogicalExpr::literal(12))
                    .add(field(h.field.as_str()).date_part("month"))
                    .sub(LogicalExpr::literal(1)),
            };

            Ok(filter(gate.clone())
                .group_by([("key".to_string(), key)], aggs)
                .sort("key")
                // ES has no inherent bucket cap here; this is headroom, not an ES-shaped limit.
                .limit(10_000))
        }
        AggType::Range(r) | AggType::DateRange(r) => {
            // One indicator column per range, counted independently — so ranges may overlap and
            // a document lands in every bucket it matches, as ES does.
            let spec = schema.get(r.field.as_str());
            let mut selects = Vec::with_capacity(r.ranges.len());
            let mut aggs = Vec::with_capacity(r.ranges.len());
            for (i, range) in r.ranges.iter().enumerate() {
                let alias = format!("{RANGE_PREFIX}{i}");
                selects.push((alias.clone(), indicator(spec, r, range)?));
                aggs.push((alias.clone(), AggregateExpr::count(Some(alias))));
            }

            Ok(filter(gate.clone())
                .select(selects)
                .group_by([("_bucket".to_string(), LogicalExpr::literal(true))], aggs))
        }
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

pub fn collect(
    schema: &Schema,
    clause: &AggClause,
    docs: Vec<Document>,
) -> Result<AggResult, Error> {
    match &clause.ty {
        AggType::Terms(terms) => {
            let terms_field = terms.field.as_str();
            let mut buckets = Vec::with_capacity(docs.len());

            for mut doc in docs {
                let raw = doc.fields.remove("key").unwrap_or_else(Value::null);
                // ES reports boolean terms keys as 1/0 with a "true"/"false" companion.
                let (key, key_as_string) = match raw.as_bool() {
                    Some(b) => (JsonValue::from(Value::i64(b as i64)), Some(b.to_string())),
                    // ES renders a date bucket key as epoch millis plus an ISO companion.
                    None if schema.get(terms_field).is_some_and(date::is_timestamp) => {
                        let iso = raw.as_timestamp().and_then(date::format_millis);
                        (JsonValue::from(raw), iso)
                    }
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
                    sub_aggs.insert(
                        name.clone(),
                        AggResult::Metric {
                            value,
                            value_as_string: None,
                        },
                    );
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
        AggType::DateHistogram(h) => {
            let interval = date::interval(h)?;
            let min_doc_count = h.min_doc_count.unwrap_or(0);
            let mut buckets = Vec::with_capacity(docs.len());

            for mut doc in docs {
                // `date_part` yields i32 while fixed-interval division yields i64, so widen
                // rather than matching one variant.
                let index = doc.fields.remove("key").and_then(|v| v.as_timestamp());
                let doc_count = doc
                    .fields
                    .remove("doc_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                if doc_count < min_doc_count {
                    continue;
                }

                let Some(key) = index.and_then(|i| date::bucket_start(&interval, i)) else {
                    continue;
                };
                let Some(key_as_string) = date::format_millis(key) else {
                    continue;
                };

                let mut sub_aggs = HashMap::new();
                for (name, _) in clause.aggs.iter().flatten() {
                    let value = doc.fields.remove(name).and_then(|v| v.number());
                    sub_aggs.insert(
                        name.clone(),
                        AggResult::Metric {
                            value,
                            value_as_string: None,
                        },
                    );
                }

                buckets.push(HistogramBucket {
                    key,
                    key_as_string,
                    doc_count,
                    sub_aggs,
                });
            }

            // ES returns date_histogram buckets in chronological order.
            buckets.sort_by_key(|b| b.key);

            Ok(AggResult::Histogram { buckets })
        }
        AggType::Range(r) | AggType::DateRange(r) => {
            let is_date = matches!(clause.ty, AggType::DateRange(_));
            let spec = schema.get(r.field.as_str());
            let mut doc = docs.into_iter().next().unwrap_or_default();
            let buckets = r
                .ranges
                .iter()
                .enumerate()
                .map(|(i, range)| {
                    let doc_count = doc
                        .fields
                        .remove(&format!("{RANGE_PREFIX}{i}"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    RangeBucket {
                        key: range.key.clone(),
                        from: bound_value(spec, range.from.as_ref()),
                        from_as_string: bound_as_string(is_date, spec, range.from.as_ref()),
                        to: bound_value(spec, range.to.as_ref()),
                        to_as_string: bound_as_string(is_date, spec, range.to.as_ref()),
                        doc_count,
                    }
                })
                .collect();

            Ok(AggResult::Range { buckets })
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

            // On a date field ES pairs the raw millis with an ISO companion. `value_count` is a
            // plain count and `terms` is not a metric, so neither renders as a date.
            let value_as_string = match (&clause.ty, value) {
                (AggType::Sum(m) | AggType::Avg(m) | AggType::Min(m) | AggType::Max(m), Some(v))
                    if schema.get(m.field.as_str()).is_some_and(date::is_timestamp) =>
                {
                    date::format_millis(v as i64)
                }
                _ => None,
            };

            Ok(AggResult::Metric {
                value,
                value_as_string,
            })
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
            AggType::Terms(_)
            | AggType::DateHistogram(_)
            | AggType::Range(_)
            | AggType::DateRange(_) => Err(Error::Unsupported(
                "Nested bucket sub-aggregations are not supported".into(),
            )),
        }
    }
}

// `1` when the document falls in the range, null otherwise, so `count` over the column is the
// bucket's doc_count. Bounds go through the same coercion as a range query, so a `date_range`
// accepts ISO strings and date math.
fn indicator(
    spec: Option<&FieldSpec>,
    body: &RangeAggBody,
    range: &RangeSpec,
) -> Result<LogicalExpr, Error> {
    let mut pred: Option<LogicalExpr> = None;
    for (bound, inclusive) in [(&range.from, true), (&range.to, false)] {
        let Some(bound) = bound else { continue };
        let bound = date::to_timestamp(spec, bound.clone().into_inner(), None)?;
        let clause = match inclusive {
            true => field(body.field.as_str()).gte(bound),
            false => field(body.field.as_str()).lt(bound),
        };
        pred = Some(match pred {
            Some(prev) => prev.and(clause),
            None => clause,
        });
    }

    // A range with neither bound matches every document.
    let pred = pred.unwrap_or_else(|| LogicalExpr::literal(true));
    Ok(pred.choose(LogicalExpr::literal(1), LogicalExpr::literal(Value::null())))
}

// ES echoes a bucket's bounds as it stored them: epoch millis for a date, plus an ISO companion.
// A date bound arrives as a string, so it goes through the same coercion the indicator used.
fn bound_value(spec: Option<&FieldSpec>, bound: Option<&JsonValue>) -> Option<f64> {
    coerce_bound(spec, bound)?.number()
}

fn bound_as_string(
    is_date: bool,
    spec: Option<&FieldSpec>,
    bound: Option<&JsonValue>,
) -> Option<String> {
    is_date
        .then(|| coerce_bound(spec, bound)?.as_timestamp().and_then(date::format_millis))
        .flatten()
}

fn coerce_bound(spec: Option<&FieldSpec>, bound: Option<&JsonValue>) -> Option<Value> {
    date::to_timestamp(spec, bound?.clone().into_inner(), None).ok()
}
