use std::collections::btree_map::BTreeMap;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::{AggregateExpr, Document, LogicalExpr, Query as TopkQuery, Value};
use topk_rs::query::{field, filter};

use super::{Schema, AVG_COUNT_PREFIX, RANGE_PREFIX};
use crate::api::{
    AggClause, AggResult, AggType, DateHistogramBody, HistogramBucket, RangeAggBody, RangeBucket,
    RangeSpec, TermsAggBody, TermsBucket,
};
use crate::date;
use crate::value::{compare, ValueExt};
use crate::Error;

// The engine caps grouped rows, and a dense histogram caps reported buckets. Both are headroom
// rather than ES-shaped limits: ES has no row cap and allows 65536 buckets.
const MAX_ROWS: u64 = 10_000;
const MAX_BUCKETS: usize = 10_000;

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
                match &sub_clause.ty {
                    // avg is folded from sum and count so buckets merged from named-zone
                    // sub-buckets stay exact — averages of averages would not be.
                    AggType::Avg(m) => {
                        aggs.push((name.clone(), AggregateExpr::sum(m.field.clone())));
                        aggs.push((
                            format!("{AVG_COUNT_PREFIX}{name}"),
                            AggregateExpr::count(Some(m.field.clone().into())),
                        ));
                    }
                    ty => aggs.push((name.clone(), AggregateExpr::try_from(ty.clone())?)),
                }
            }

            let key = date::bucketing(h)?.key_expr(h.field.as_str());

            Ok(filter(gate.clone())
                .group_by([("key".to_string(), key)], aggs)
                .sort("key")
                .limit(MAX_ROWS))
        }
        AggType::Range(r) | AggType::DateRange(r) => {
            // One indicator column per range, counted independently — so ranges may overlap and
            // a document lands in every bucket it matches, as ES does.
            let spec = schema.get(r.field.as_str());
            let zone = r.time_zone.as_deref().map(date::Zone::parse).transpose()?;
            let mut selects = Vec::with_capacity(r.ranges.len());
            let mut aggs = Vec::with_capacity(r.ranges.len());
            for (i, range) in r.ranges.iter().enumerate() {
                let alias = format!("{RANGE_PREFIX}{i}");
                selects.push((alias.clone(), indicator(spec, zone.as_ref(), r, range)?));
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

// Engine rows may be finer than the reported buckets (see `date::Bucketing`), so they are merged
// here before the empty ones are filled in.
fn histogram(
    schema: &Schema,
    clause: &AggClause,
    h: &DateHistogramBody,
    docs: Vec<Document>,
) -> Result<AggResult, Error> {
    let bucketing = date::bucketing(h)?;
    let zone = h.time_zone.as_deref().map(date::Zone::parse).transpose()?;
    let min_doc_count = h.min_doc_count.unwrap_or(0);
    let spec = schema.get(h.field.as_str());

    let mut merged = merge_rows(clause, &bucketing, docs);

    let bound_millis = |bound: &Option<JsonValue>| -> Result<Option<i64>, Error> {
        let Some(bound) = bound else { return Ok(None) };
        let value = date::to_timestamp(spec, bound.clone().into_inner(), zone.as_ref())?;
        Ok(value.as_timestamp())
    };

    // `hard_bounds` trims whole buckets, from the window's lower edge up to but excluding its
    // upper one; it never extends. ES rounds those edges ignoring `offset`, unlike
    // `extended_bounds`.
    if let Some(hard) = &h.hard_bounds {
        let min = bound_millis(&hard.min)?.and_then(|t| bucketing.floor_unshifted(t));
        let max = bound_millis(&hard.max)?.and_then(|t| bucketing.floor_unshifted(t));
        merged.retain(|key, _| min.is_none_or(|m| *key >= m) && max.is_none_or(|m| *key < m));
    }

    // With `min_doc_count: 0` (the ES default) the histogram is dense: every bucket between the
    // first and last, stretched to `extended_bounds`, is reported with the empty ones included.
    let mut buckets = match min_doc_count {
        0 => {
            let mut lo = merged.first_key_value().map(|(k, _)| *k);
            let mut hi = merged.last_key_value().map(|(k, _)| *k);
            for value in h.extended_bounds.iter().flat_map(|b| [&b.min, &b.max]) {
                let Some(key) = bound_millis(value)?.and_then(|t| bucketing.extended_bound(t))
                else {
                    continue;
                };
                lo = Some(lo.map_or(key, |lo| lo.min(key)));
                hi = Some(hi.map_or(key, |hi| hi.max(key)));
            }
            fill(schema, clause, zone.as_ref(), &bucketing, merged, lo, hi)?
        }
        min => merged
            .into_iter()
            .filter(|(_, (doc_count, _))| *doc_count >= min)
            .filter_map(|(key, (doc_count, metrics))| {
                bucket(schema, clause, zone.as_ref(), key, doc_count, metrics)
            })
            .collect(),
    };

    // Buckets are built chronologically; `order` reorders at the end.
    if let Some(order) = &h.order {
        reorder(&mut buckets, order)?;
    }

    Ok(AggResult::Histogram { buckets })
}

fn fill(
    schema: &Schema,
    clause: &AggClause,
    zone: Option<&date::Zone>,
    bucketing: &date::Bucketing,
    mut merged: Merged,
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

// ES orders `date_histogram` buckets by `_key` or `_count`; it also accepts a sub-agg name, which
// we do not.
fn reorder(buckets: &mut [HistogramBucket], order: &HashMap<String, String>) -> Result<(), Error> {
    let (target, direction) = order
        .iter()
        .next()
        .ok_or_else(|| Error::BadRequest("date_histogram order must name a target".into()))?;

    let descending = match direction.as_str() {
        "asc" => false,
        "desc" => true,
        _ => {
            return Err(Error::BadRequest(format!(
                "invalid date_histogram order direction [{direction}]"
            )))
        }
    };

    match (target.as_str(), descending) {
        ("_key", false) => {}
        ("_key", true) => buckets.reverse(),
        // ES breaks `doc_count` ties by key, ascending.
        ("_count", false) => {
            buckets.sort_by(|a, b| a.doc_count.cmp(&b.doc_count).then(a.key.cmp(&b.key)))
        }
        ("_count", true) => {
            buckets.sort_by(|a, b| b.doc_count.cmp(&a.doc_count).then(a.key.cmp(&b.key)))
        }
        _ => {
            return Err(Error::Unsupported(
                "date_histogram order supports _key and _count".into(),
            ))
        }
    }

    Ok(())
}

// Sum the engine rows into one entry per reported bucket. A named zone buckets at a finer
// granularity than it reports, so several rows can fold onto the same start. The BTreeMap keeps
// buckets chronological, as ES returns them.
fn merge_rows(clause: &AggClause, bucketing: &date::Bucketing, docs: Vec<Document>) -> Merged {
    let mut merged: Merged = BTreeMap::new();
    for mut doc in docs {
        let Some(start) = doc
            .fields
            .remove("key")
            .and_then(|v| v.as_timestamp())
            .and_then(|index| bucketing.start_of_index(index))
        else {
            continue;
        };

        let (doc_count, metrics) = merged.entry(start).or_default();
        *doc_count += doc
            .fields
            .remove("doc_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        for (name, sub_clause) in clause.aggs.iter().flatten() {
            let value = doc.fields.remove(name).and_then(|v| v.number());
            let acc = match (&sub_clause.ty, value) {
                (AggType::Avg(_), value) => MetricAcc::Ratio(
                    value.unwrap_or(0.0),
                    doc.fields
                        .remove(&format!("{AVG_COUNT_PREFIX}{name}"))
                        .and_then(|v| v.number())
                        .unwrap_or(0.0),
                ),
                (_, None) => continue,
                (AggType::Min(_), Some(value)) => MetricAcc::Min(value),
                (AggType::Max(_), Some(value)) => MetricAcc::Max(value),
                (_, Some(value)) => MetricAcc::Add(value),
            };
            match metrics.entry(name.clone()) {
                Entry::Vacant(slot) => {
                    slot.insert(acc);
                }
                Entry::Occupied(mut slot) => slot.get_mut().fold(acc),
            }
        }
    }

    merged
}

// Reported bucket start -> its doc count and folded sub-agg metrics.
type Merged = BTreeMap<i64, (u64, HashMap<String, MetricAcc>)>;

// One bucket per requested range, counted from the indicator columns `compile` selected.
fn ranges(schema: &Schema, r: &RangeAggBody, docs: Vec<Document>) -> Result<AggResult, Error> {
    let spec = schema.get(r.field.as_str());
    let zone = r.time_zone.as_deref().map(date::Zone::parse).transpose()?;
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
            let (from, from_as_string) = bound(spec, zone.as_ref(), range.from.as_ref());
            let (to, to_as_string) = bound(spec, zone.as_ref(), range.to.as_ref());
            // ES synthesizes "from-to" keys ("*" for an open side) when none is given:
            // the formatted date for a date field, `100.0`-style numbers otherwise.
            let key = range.key.clone().unwrap_or_else(|| {
                let side = |value: Option<f64>, s: &Option<String>| match (s, value) {
                    (Some(s), _) => s.clone(),
                    (None, Some(v)) => format!("{v:?}"),
                    (None, None) => "*".to_string(),
                };
                format!(
                    "{}-{}",
                    side(from, &from_as_string),
                    side(to, &to_as_string)
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
        let from = |x: &RangeBucket| x.from.unwrap_or(f64::NEG_INFINITY);
        let to = |x: &RangeBucket| x.to.unwrap_or(f64::INFINITY);
        from(a).total_cmp(&from(b)).then(to(a).total_cmp(&to(b)))
    });

    Ok(AggResult::Range { buckets })
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

// `1` when the document falls in the range, null otherwise, so `count` over the column is the
// bucket's doc_count. Bounds go through the same coercion as a range query, so a date field
// accepts ISO strings and date math.
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

// ES echoes a bucket's bounds as it stored them: epoch millis, plus an ISO companion rendered in
// the request zone when the field is a date. Verified on ES 8.15 that `range` and `date_range`
// agree here, so the companion follows the mapping and not the aggregation name.
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

// On a date field ES pairs the raw millis with an ISO companion. `value_count` is a plain count
// and never renders as a date.
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

// A sub-agg metric folded across the engine rows that merged into one bucket.
enum MetricAcc {
    Add(f64),
    // avg as numerator/denominator; see `compile`.
    Ratio(f64, f64),
    Min(f64),
    Max(f64),
}

impl MetricAcc {
    fn fold(&mut self, next: MetricAcc) {
        match (self, next) {
            (Self::Add(a), Self::Add(b)) => *a += b,
            (Self::Ratio(s, c), Self::Ratio(s2, c2)) => {
                *s += s2;
                *c += c2;
            }
            (Self::Min(a), Self::Min(b)) => *a = a.min(b),
            (Self::Max(a), Self::Max(b)) => *a = a.max(b),
            _ => {}
        }
    }

    fn value(&self) -> Option<f64> {
        match self {
            Self::Add(v) | Self::Min(v) | Self::Max(v) => Some(*v),
            Self::Ratio(sum, count) => (*count > 0.0).then(|| sum / count),
        }
    }
}

// `None` when the key does not format, which only happens far outside the representable range.
fn bucket(
    schema: &Schema,
    clause: &AggClause,
    zone: Option<&date::Zone>,
    key: i64,
    doc_count: u64,
    metrics: HashMap<String, MetricAcc>,
) -> Option<HistogramBucket> {
    let key_as_string = date::format(key, zone)?;

    let mut sub_aggs = HashMap::new();
    for (name, sub_clause) in clause.aggs.iter().flatten() {
        let value = match metrics.get(name) {
            Some(acc) => acc.value(),
            // An empty bucket sums and counts to 0; avg/min/max stay null, as in ES.
            None => {
                matches!(sub_clause.ty, AggType::Sum(_) | AggType::ValueCount(_)).then_some(0.0)
            }
        };
        sub_aggs.insert(name.clone(), metric_result(schema, &sub_clause.ty, value));
    }

    Some(HistogramBucket {
        key,
        key_as_string,
        doc_count,
        sub_aggs,
    })
}
