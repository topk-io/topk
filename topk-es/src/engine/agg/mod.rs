use std::collections::btree_map::BTreeMap;
use std::collections::HashMap;

use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::data::{AggregateExpr, Document, LogicalExpr, Query as TopkQuery, Value};
use topk_rs::query::{field, filter};

use super::Ctx;
use crate::api::{
    AggClause, AggResult, AggType, Bounds, DateHistogramBody, HistogramBucket, TermsBucket,
};
use crate::date::{self, Interval, Round, Zone};
use crate::value::{compare, ValueExt};
use crate::Error;

const MAX_BUCKETS: usize = 10_000;

pub struct AggPlan {
    pub query: TopkQuery,
    shape: Shape,
}

enum Shape {
    Metric(Metric),
    Terms { iso: bool, subs: Subs },
    Histogram(Histogram),
}

#[derive(Clone, Copy)]
struct Metric {
    zero_when_empty: bool,
    iso: bool,
}

type Subs = Vec<(String, Metric)>;

struct Histogram {
    interval: Interval,
    zone: Option<Zone>,
    min_doc_count: u64,
    extended: (Option<i64>, Option<i64>),
    subs: Subs,
}

pub fn plan(ctx: &Ctx, clause: &AggClause, gate: &LogicalExpr) -> Result<AggPlan, Error> {
    let bucket_aggs = || {
        let mut aggs = vec![("doc_count".to_string(), AggregateExpr::count(None))];
        for (name, sub) in clause.aggs.iter().flatten() {
            aggs.push((name.clone(), AggregateExpr::try_from(sub.ty.clone())?));
        }
        Ok::<_, Error>(aggs)
    };

    match &clause.ty {
        AggType::Terms(terms) => Ok(AggPlan {
            query: filter(gate.clone())
                .group_by(
                    [("key".to_string(), field(terms.field.as_str()))],
                    bucket_aggs()?,
                )
                .sort("doc_count")
                .limit(terms.size.unwrap_or(10) as u64),
            shape: Shape::Terms {
                iso: is_date(ctx, terms.field.as_str()),
                subs: subs(ctx, clause),
            },
        }),

        AggType::DateHistogram(h) => {
            let histogram = histogram(ctx, h, subs(ctx, clause))?;
            let key = histogram.interval.key_expr(
                h.field.as_str(),
                histogram.zone.as_ref().unwrap_or(&Zone::UTC),
            );
            let query = filter(gate.clone()).group_by([("key".to_string(), key)], bucket_aggs()?);

            let query = match histogram.min_doc_count {
                0 => query,
                min => query.filter(field("doc_count").gte(LogicalExpr::literal(min as i64))),
            };

            Ok(AggPlan {
                query: query.sort("key").limit(MAX_BUCKETS as u64),
                shape: Shape::Histogram(histogram),
            })
        }

        metric => Ok(AggPlan {
            query: filter(gate.clone()).group_by(
                [("_bucket".to_string(), LogicalExpr::literal(true))],
                [(
                    "value".to_string(),
                    AggregateExpr::try_from(metric.clone())?,
                )],
            ),
            shape: Shape::Metric(metric_of(ctx, metric)),
        }),
    }
}

pub fn collect(plan: &AggPlan, docs: Vec<Document>) -> Result<AggResult, Error> {
    match &plan.shape {
        Shape::Metric(m) => Ok(m.render(
            docs.into_iter()
                .next()
                .and_then(|mut doc| doc.fields.remove("value"))
                .and_then(|v| v.number()),
        )),
        Shape::Terms { iso, subs } => Ok(terms(*iso, subs, docs)),
        Shape::Histogram(h) => histogram_result(h, docs),
    }
}

impl Metric {
    fn render(&self, value: Option<f64>) -> AggResult {
        let value = match (value, self.zero_when_empty) {
            (None, true) => Some(0.0),
            (value, _) => value,
        };
        AggResult::Metric {
            value_as_string: match (self.iso, value) {
                (true, Some(v)) => date::format(v as i64, None),
                _ => None,
            },
            value,
        }
    }
}

fn metric_of(ctx: &Ctx, ty: &AggType) -> Metric {
    Metric {
        zero_when_empty: matches!(ty, AggType::Sum(_) | AggType::ValueCount(_)),
        iso: match ty {
            AggType::Sum(m) | AggType::Avg(m) | AggType::Min(m) | AggType::Max(m) => {
                is_date(ctx, m.field.as_str())
            }
            _ => false,
        },
    }
}

fn subs(ctx: &Ctx, clause: &AggClause) -> Subs {
    clause
        .aggs
        .iter()
        .flatten()
        .map(|(name, sub)| (name.clone(), metric_of(ctx, &sub.ty)))
        .collect()
}

fn is_date(ctx: &Ctx, field: &str) -> bool {
    ctx.schema.get(field).is_some_and(date::is_timestamp)
}

fn sub_results(
    subs: &Subs,
    mut value: impl FnMut(&str) -> Option<f64>,
) -> HashMap<String, AggResult> {
    subs.iter()
        .map(|(name, metric)| (name.clone(), metric.render(value(name))))
        .collect()
}

fn doc_count(doc: &mut Document) -> u64 {
    doc.fields
        .remove("doc_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

fn terms(iso: bool, subs: &Subs, docs: Vec<Document>) -> AggResult {
    let mut buckets: Vec<TermsBucket> = docs
        .into_iter()
        .map(|mut doc| {
            let raw = doc.fields.remove("key").unwrap_or_else(Value::null);
            let (key, key_as_string) = match (raw.as_bool(), iso) {
                (Some(b), _) => (JsonValue::from(Value::i64(b as i64)), Some(b.to_string())),
                (None, true) => {
                    let iso = raw.as_timestamp().and_then(|t| date::format(t, None));
                    (JsonValue::from(raw), iso)
                }
                (None, false) => (JsonValue::from(raw), None),
            };

            TermsBucket {
                key,
                key_as_string,
                doc_count: doc_count(&mut doc),
                sub_aggs: sub_results(subs, |name| {
                    doc.fields.remove(name).and_then(|v| v.number())
                }),
            }
        })
        .collect();

    buckets.sort_by(|a, b| {
        b.doc_count
            .cmp(&a.doc_count)
            .then_with(|| compare(&a.key, &b.key))
    });

    AggResult::Terms {
        doc_count_error_upper_bound: 0,
        sum_other_doc_count: 0,
        buckets,
    }
}

fn histogram(ctx: &Ctx, h: &DateHistogramBody, subs: Subs) -> Result<Histogram, Error> {
    let interval = match (&h.fixed_interval, &h.calendar_interval) {
        (Some(_), Some(_)) => {
            return Err(Error::BadRequest(
                "date_histogram accepts either fixed_interval or calendar_interval, not both"
                    .into(),
            ))
        }
        (Some(fixed), None) => Interval::Fixed(date::parse_fixed_interval(fixed)?),
        (None, Some(calendar)) => Interval::Calendar(date::parse_calendar_interval(calendar)?),
        (None, None) => {
            return Err(Error::BadRequest(
                "date_histogram requires fixed_interval or calendar_interval".into(),
            ))
        }
    };

    let spec = ctx.schema.get(h.field.as_str());
    let edge = |bound: &Option<JsonValue>| match bound {
        None => Ok(None),
        Some(b) => date::to_timestamp(
            spec,
            b.clone().into_inner(),
            h.time_zone.as_ref(),
            Round::Down,
            ctx.now,
        )
        .map(|v| v.as_timestamp()),
    };
    let extended = match &h.extended_bounds {
        None => (None, None),
        Some(Bounds { min, max }) => (edge(min)?, edge(max)?),
    };

    Ok(Histogram {
        interval,
        zone: h.time_zone,
        min_doc_count: h.min_doc_count.unwrap_or(0),
        extended,
        subs,
    })
}

fn histogram_result(h: &Histogram, docs: Vec<Document>) -> Result<AggResult, Error> {
    let mut merged: BTreeMap<i64, (u64, HashMap<String, f64>)> = docs
        .into_iter()
        .filter_map(|mut doc| {
            let start = doc.fields.remove("key")?.as_timestamp()?;
            let count = doc_count(&mut doc);
            let metrics = h
                .subs
                .iter()
                .filter_map(|(name, _)| Some((name.clone(), doc.fields.remove(name)?.number()?)))
                .collect();
            Some((start, (count, metrics)))
        })
        .collect();

    let buckets = match h.min_doc_count {
        0 => {
            let mut lo = merged.first_key_value().map(|(k, _)| *k);
            let mut hi = merged.last_key_value().map(|(k, _)| *k);
            for at in [h.extended.0, h.extended.1].into_iter().flatten() {
                lo = Some(lo.map_or(at, |lo| lo.min(at)));
                hi = Some(hi.map_or(at, |hi| hi.max(at)));
            }

            let mut buckets = Vec::new();
            if let Some((mut start, hi)) = lo.zip(hi) {
                loop {
                    if buckets.len() >= MAX_BUCKETS {
                        return Err(Error::BadRequest(format!(
                            "date_histogram would produce more than {MAX_BUCKETS} buckets"
                        )));
                    }
                    let bucket = merged.remove(&start).unwrap_or_default();
                    buckets.push(bucket_at(h, start, bucket)?);

                    match h.interval.next(start, h.zone.as_ref()) {
                        Some(next) if next <= hi => start = next,
                        _ => break,
                    }
                }
            }
            buckets
        }
        _ => merged
            .into_iter()
            .map(|(start, bucket)| bucket_at(h, start, bucket))
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(AggResult::Histogram { buckets })
}

fn bucket_at(
    h: &Histogram,
    key: i64,
    (doc_count, metrics): (u64, HashMap<String, f64>),
) -> Result<HistogramBucket, Error> {
    Ok(HistogramBucket {
        key_as_string: date::format(key, h.zone.as_ref()).ok_or_else(|| {
            Error::BadRequest(format!("date_histogram bucket [{key}] is out of range"))
        })?,
        key,
        doc_count,
        sub_aggs: sub_results(&h.subs, |name| metrics.get(name).copied()),
    })
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
            AggType::Terms(_) | AggType::DateHistogram(_) => Err(Error::Unsupported(
                "Nested bucket sub-aggregations are not supported".into(),
            )),
        }
    }
}
