use std::collections::btree_map::BTreeMap;
use std::collections::HashMap;

use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::{AggregateExpr, Document, LogicalExpr, Query as TopkQuery, Value};
use topk_rs::query::{field, filter};

use super::{Schema, RANGE_PREFIX};
use crate::api::{
    AggClause, AggResult, AggType, Bounds, DateHistogramBody, Direction, HistogramBucket, Order,
    OrderTarget, RangeAggBody, RangeBucket, RangeSpec, TermsAggBody, TermsBucket,
};
use crate::date;
use crate::value::{compare, ValueExt};
use crate::Error;

const MAX_BUCKETS: usize = 65_536;

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

            let key = date::bucketing(h.interval, h.offset, h.shift).key_expr(h.field.as_str());

            Ok(filter(gate.clone())
                .group_by([("key".to_string(), key)], aggs)
                .sort("key")
                .limit(MAX_BUCKETS as u64))
        }
        AggType::Range(r) | AggType::DateRange(r) => {
            let spec = schema.get(r.field.as_str());
            let zone = r.time_zone.as_ref();
            let mut selects = Vec::with_capacity(r.ranges.len());
            let mut aggs = Vec::with_capacity(r.ranges.len());
            for (i, range) in r.ranges.iter().enumerate() {
                let alias = format!("{RANGE_PREFIX}{i}");
                selects.push((alias.clone(), indicator(spec, zone, r, range)?));
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
        AggType::Terms(t) => terms(schema, clause, t, docs),
        AggType::DateHistogram(h) => histogram(schema, clause, h, docs),
        AggType::Range(r) | AggType::DateRange(r) => ranges(schema, r, docs),
        ty => metric(schema, ty, docs),
    }
}

fn terms(
    schema: &Schema,
    clause: &AggClause,
    terms: &TermsAggBody,
    docs: Vec<Document>,
) -> Result<AggResult, Error> {
    let terms_field = terms.field.as_str();
    let mut buckets = Vec::with_capacity(docs.len());

    for mut doc in docs {
        let raw = doc.fields.remove("key").unwrap_or_else(Value::null);
        // ES reports boolean terms keys as 1/0 with a "true"/"false" companion.
        let (key, key_as_string) = match raw.as_bool() {
            Some(b) => (JsonValue::from(Value::i64(b as i64)), Some(b.to_string())),
            // ES renders a date bucket key as epoch millis plus an ISO companion.
            None if schema.get(terms_field).is_some_and(date::is_timestamp) => {
                let iso = raw.as_timestamp().and_then(|t| date::format(t, None));
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
        for (name, sub_clause) in clause.aggs.iter().flatten() {
            let value = doc.fields.remove(name).and_then(|v| v.number());
            sub_aggs.insert(name.clone(), metric_result(schema, &sub_clause.ty, value));
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

fn histogram(
    schema: &Schema,
    clause: &AggClause,
    h: &DateHistogramBody,
    docs: Vec<Document>,
) -> Result<AggResult, Error> {
    let bucketing = date::bucketing(h.interval, h.offset, h.shift);
    let zone = h.zone.as_ref();
    let spec = schema.get(h.field.as_str());

    let mut merged = by_bucket(clause, &bucketing, docs);

    let (hard_min, hard_max) = bounds_millis(spec, zone, h.hard_bounds.as_ref())?;
    let (ext_min, ext_max) = bounds_millis(spec, zone, h.extended_bounds.as_ref())?;

    if ext_min.zip(hard_min).is_some_and(|(ext, hard)| ext < hard)
        || ext_max.zip(hard_max).is_some_and(|(ext, hard)| ext > hard)
    {
        return Err(Error::BadRequest(
            "extended_bounds have to be inside hard_bounds".into(),
        ));
    }

    if h.hard_bounds.is_some() {
        let min = hard_min.and_then(|t| bucketing.floor_unshifted(t));
        let max = hard_max.and_then(|t| bucketing.floor_unshifted(t));
        merged.retain(|key, _| min.is_none_or(|m| *key >= m) && max.is_none_or(|m| *key < m));
    }

    let mut buckets = match h.min_doc_count {
        0 => {
            let mut lo = merged.first_key_value().map(|(k, _)| *k);
            let mut hi = merged.last_key_value().map(|(k, _)| *k);
            for key in [ext_min, ext_max]
                .into_iter()
                .flatten()
                .filter_map(|t| bucketing.extended_bound(t))
            {
                lo = Some(lo.map_or(key, |lo| lo.min(key)));
                hi = Some(hi.map_or(key, |hi| hi.max(key)));
            }
            fill(schema, clause, zone, &bucketing, merged, lo, hi)?
        }
        min => merged
            .into_iter()
            .filter(|(_, (doc_count, _))| *doc_count >= min)
            .filter_map(|(key, (doc_count, metrics))| {
                bucket(schema, clause, zone, key, doc_count, metrics)
            })
            .collect(),
    };

    // Buckets are built chronologically; `order` reorders at the end.
    if let Some(order) = h.order {
        reorder(&mut buckets, order);
    }

    Ok(AggResult::Histogram { buckets })
}

fn fill(
    schema: &Schema,
    clause: &AggClause,
    zone: Option<&date::Zone>,
    bucketing: &date::Bucketing,
    mut merged: Buckets,
    lo: Option<i64>,
    hi: Option<i64>,
) -> Result<Vec<HistogramBucket>, Error> {
    let (Some(mut key), Some(hi)) = (lo, hi) else {
        return Ok(Vec::new());
    };

    let mut buckets = Vec::new();
    loop {
        if buckets.len() >= MAX_BUCKETS {
            return Err(Error::BadRequest(format!(
                "date_histogram would produce more than {MAX_BUCKETS} buckets"
            )));
        }
        let (doc_count, metrics) = merged.remove(&key).unwrap_or_default();
        buckets.extend(bucket(schema, clause, zone, key, doc_count, metrics));
        match bucketing.next(key) {
            Some(next) if next <= hi => key = next,
            _ => break Ok(buckets),
        }
    }
}

fn reorder(buckets: &mut [HistogramBucket], order: Order) {
    match (order.target, order.direction) {
        (OrderTarget::Key, Direction::Asc) => {}
        (OrderTarget::Key, Direction::Desc) => buckets.reverse(),
        // ES breaks `doc_count` ties by key, ascending.
        (OrderTarget::Count, Direction::Asc) => {
            buckets.sort_by(|a, b| a.doc_count.cmp(&b.doc_count).then(a.key.cmp(&b.key)))
        }
        (OrderTarget::Count, Direction::Desc) => {
            buckets.sort_by(|a, b| b.doc_count.cmp(&a.doc_count).then(a.key.cmp(&b.key)))
        }
    }
}

fn by_bucket(clause: &AggClause, bucketing: &date::Bucketing, docs: Vec<Document>) -> Buckets {
    let mut buckets: Buckets = BTreeMap::new();
    for mut doc in docs {
        let Some(start) = doc
            .fields
            .remove("key")
            .and_then(|v| v.as_timestamp())
            .and_then(|index| bucketing.start_of_index(index))
        else {
            continue;
        };

        let doc_count = doc
            .fields
            .remove("doc_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let metrics = clause
            .aggs
            .iter()
            .flatten()
            .filter_map(|(name, _)| {
                let value = doc.fields.remove(name).and_then(|v| v.number())?;
                Some((name.clone(), value))
            })
            .collect();

        buckets.insert(start, (doc_count, metrics));
    }

    buckets
}

type Buckets = BTreeMap<i64, (u64, HashMap<String, f64>)>;

// One bucket per requested range, counted from the indicator columns `compile` selected.
fn ranges(schema: &Schema, r: &RangeAggBody, docs: Vec<Document>) -> Result<AggResult, Error> {
    let spec = schema.get(r.field.as_str());
    let zone = r.time_zone.as_ref();
    let mut doc = docs.into_iter().next().unwrap_or_default();
    let mut buckets: Vec<RangeBucket> = r
        .ranges
        .iter()
        .enumerate()
        .map(|(i, range)| {
            let doc_count = doc
                .fields
                .remove(&format!("{RANGE_PREFIX}{i}"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let (from, from_as_string) = bound(spec, zone, range.from.as_ref());
            let (to, to_as_string) = bound(spec, zone, range.to.as_ref());
            let key = range.key.clone().unwrap_or_else(|| {
                format!(
                    "{}-{}",
                    key_side(from, &from_as_string),
                    key_side(to, &to_as_string)
                )
            });
            RangeBucket {
                key,
                from,
                from_as_string,
                to,
                to_as_string,
                doc_count,
            }
        })
        .collect();

    // ES orders range buckets by bounds — an unbounded `from` first — not as defined.
    buckets.sort_by(|a, b| {
        a.from
            .unwrap_or(f64::NEG_INFINITY)
            .total_cmp(&b.from.unwrap_or(f64::NEG_INFINITY))
            .then(
                a.to.unwrap_or(f64::INFINITY)
                    .total_cmp(&b.to.unwrap_or(f64::INFINITY)),
            )
    });

    Ok(AggResult::Range { buckets })
}

fn bounds_millis(
    spec: Option<&FieldSpec>,
    zone: Option<&date::Zone>,
    bounds: Option<&Bounds>,
) -> Result<(Option<i64>, Option<i64>), Error> {
    let Some(bounds) = bounds else {
        return Ok((None, None));
    };
    Ok((
        bound_millis(spec, zone, &bounds.min)?,
        bound_millis(spec, zone, &bounds.max)?,
    ))
}

fn bound_millis(
    spec: Option<&FieldSpec>,
    zone: Option<&date::Zone>,
    bound: &Option<JsonValue>,
) -> Result<Option<i64>, Error> {
    let Some(bound) = bound else { return Ok(None) };
    Ok(date::to_timestamp(spec, bound.clone().into_inner(), zone)?.as_timestamp())
}

fn key_side(value: Option<f64>, as_string: &Option<String>) -> String {
    match (as_string, value) {
        (Some(s), _) => s.clone(),
        (None, Some(v)) => format!("{v:?}"),
        (None, None) => "*".to_string(),
    }
}

fn metric(schema: &Schema, ty: &AggType, docs: Vec<Document>) -> Result<AggResult, Error> {
    let value = docs
        .into_iter()
        .next()
        .and_then(|mut doc| doc.fields.remove("value"))
        .and_then(|v| v.number());

    // Over an empty match set ES sums and counts to 0; avg/min/max stay null.
    let value = match (value, ty) {
        (None, AggType::Sum(_) | AggType::ValueCount(_)) => Some(0.0),
        (value, _) => value,
    };

    Ok(metric_result(schema, ty, value))
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

fn indicator(
    spec: Option<&FieldSpec>,
    zone: Option<&date::Zone>,
    body: &RangeAggBody,
    range: &RangeSpec,
) -> Result<LogicalExpr, Error> {
    let mut pred: Option<LogicalExpr> = None;
    for (bound, inclusive) in [(&range.from, true), (&range.to, false)] {
        let Some(bound) = bound else { continue };
        let bound = date::to_timestamp(spec, bound.clone().into_inner(), zone)?;
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

fn bound(
    spec: Option<&FieldSpec>,
    zone: Option<&date::Zone>,
    bound: Option<&JsonValue>,
) -> (Option<f64>, Option<String>) {
    let value = bound.and_then(|b| date::to_timestamp(spec, b.clone().into_inner(), zone).ok());
    let as_string = match spec.is_some_and(date::is_timestamp) {
        true => value
            .as_ref()
            .and_then(|v| v.as_timestamp())
            .and_then(|t| date::format(t, zone)),
        false => None,
    };
    (value.and_then(|v| v.number()), as_string)
}

fn metric_result(schema: &Schema, ty: &AggType, value: Option<f64>) -> AggResult {
    let value_as_string = match (ty, value) {
        (AggType::Sum(m) | AggType::Avg(m) | AggType::Min(m) | AggType::Max(m), Some(v))
            if schema.get(m.field.as_str()).is_some_and(date::is_timestamp) =>
        {
            date::format(v as i64, None)
        }
        _ => None,
    };
    AggResult::Metric {
        value,
        value_as_string,
    }
}

// `None` when the key does not format, which only happens far outside the representable range.
fn bucket(
    schema: &Schema,
    clause: &AggClause,
    zone: Option<&date::Zone>,
    key: i64,
    doc_count: u64,
    metrics: HashMap<String, f64>,
) -> Option<HistogramBucket> {
    let key_as_string = date::format(key, zone)?;

    let mut sub_aggs = HashMap::new();
    for (name, sub_clause) in clause.aggs.iter().flatten() {
        let value = metrics.get(name).copied().or_else(|| {
            matches!(sub_clause.ty, AggType::Sum(_) | AggType::ValueCount(_)).then_some(0.0)
        });
        sub_aggs.insert(name.clone(), metric_result(schema, &sub_clause.ty, value));
    }

    Some(HistogramBucket {
        key,
        key_as_string,
        doc_count,
        sub_aggs,
    })
}
