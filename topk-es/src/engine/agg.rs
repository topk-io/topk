use std::collections::btree_map::BTreeMap;

use chrono::DateTime;
use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::data::{AggregateExpr, Document, LogicalExpr, Query as TopkQuery, Value};
use topk_rs::query::{field, filter};

use super::Schema;
use crate::api::{AggClause, AggResult, AggType, Bucket, Buckets, DateHistogramBody};
use crate::date::{self, Unit, Zone};
use crate::value::{compare, ValueExt};
use crate::Error;

const MAX_BUCKETS: usize = 10_000;

#[derive(Clone, Copy)]
enum Interval {
    Calendar(Unit),
    Fixed(i64),
}

pub fn compile(clause: &AggClause, gate: &LogicalExpr) -> Result<TopkQuery, Error> {
    let bucket_aggs = || {
        let mut aggs = vec![("doc_count".to_string(), AggregateExpr::count(None))];
        for (name, sub) in &clause.aggs {
            aggs.push((name.clone(), AggregateExpr::try_from(sub.ty.clone())?));
        }
        Ok::<_, Error>(aggs)
    };

    match &clause.ty {
        AggType::Terms(terms) => Ok(filter(gate.clone())
            .group_by(
                [("key".to_string(), field(terms.field.as_str()))],
                bucket_aggs()?,
            )
            .sort("doc_count")
            .limit(terms.size.unwrap_or(10) as u64)),

        AggType::DateHistogram(h) => {
            let key = match interval(h)? {
                Interval::Calendar(unit) => {
                    field(h.field.as_str()).date_trunc(unit.to_string(), h.time_zone.to_string())
                }
                Interval::Fixed(width) => {
                    let offset =
                        LogicalExpr::literal(h.time_zone.fixed_offset_millis().unwrap_or(0));
                    field(h.field.as_str())
                        .add(offset.clone())
                        .div(LogicalExpr::literal(width))
                        .mul(LogicalExpr::literal(width))
                        .sub(offset)
                }
            };

            Ok(filter(gate.clone())
                .group_by([("key".to_string(), key)], bucket_aggs()?)
                .sort("key")
                .limit(MAX_BUCKETS as u64))
        }

        metric => Ok(filter(gate.clone()).group_by(
            [("_bucket".to_string(), LogicalExpr::literal(true))],
            [(
                "value".to_string(),
                AggregateExpr::try_from(metric.clone())?,
            )],
        )),
    }
}

pub fn collect(
    schema: &Schema,
    clause: &AggClause,
    docs: Vec<Document>,
) -> Result<AggResult, Error> {
    match &clause.ty {
        AggType::Terms(t) => Ok(terms(schema, clause, t.field.as_str(), docs)),
        AggType::DateHistogram(h) => histogram(schema, clause, h, docs),
        ty => Ok(metric(
            schema,
            ty,
            docs.into_iter()
                .next()
                .and_then(|mut doc| doc.fields.remove("value"))
                .and_then(|v| v.number()),
        )),
    }
}

fn metric(schema: &Schema, ty: &AggType, value: Option<f64>) -> AggResult {
    // Over an empty match set ES sums and counts to 0; avg/min/max stay null.
    let value = match (value, ty) {
        (None, AggType::Sum(_) | AggType::ValueCount(_) | AggType::Cardinality(_)) => Some(0.0),
        (value, _) => value,
    };
    let iso = match ty {
        AggType::Sum(m) | AggType::Avg(m) | AggType::Min(m) | AggType::Max(m) => {
            schema.get(m.field.as_str()).is_some_and(date::is_timestamp)
        }
        _ => false,
    };
    AggResult::Metric {
        value_as_string: value
            .filter(|_| iso)
            .and_then(|v| date::format(v as i64, &Zone::default())),
        value,
    }
}

fn bucket(
    schema: &Schema,
    clause: &AggClause,
    key: JsonValue,
    key_as_string: Option<String>,
    mut doc: Document,
) -> Bucket {
    Bucket {
        key,
        key_as_string,
        doc_count: doc
            .fields
            .remove("doc_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        sub_aggs: clause
            .aggs
            .iter()
            .map(|(name, sub)| {
                let value = doc.fields.remove(name).and_then(|v| v.number());
                (name.clone(), metric(schema, &sub.ty, value))
            })
            .collect(),
    }
}

fn terms(schema: &Schema, clause: &AggClause, field: &str, docs: Vec<Document>) -> AggResult {
    let iso = schema.get(field).is_some_and(date::is_timestamp);
    let mut buckets: Vec<Bucket> = docs
        .into_iter()
        .map(|mut doc| {
            let raw = doc.fields.remove("key").unwrap_or_else(Value::null);
            // ES reports boolean terms keys as 1/0 with a "true"/"false" companion.
            let (key, key_as_string) = match (raw.as_bool(), iso) {
                (Some(b), _) => (JsonValue::from(Value::i64(b as i64)), Some(b.to_string())),
                (None, true) => {
                    let iso = raw
                        .as_timestamp()
                        .and_then(|t| date::format(t, &Zone::default()));
                    (JsonValue::from(raw), iso)
                }
                (None, false) => (JsonValue::from(raw), None),
            };
            bucket(schema, clause, key, key_as_string, doc)
        })
        .collect();

    // ES breaks `doc_count` ties by key, ascending.
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

fn interval(h: &DateHistogramBody) -> Result<Interval, Error> {
    match (&h.fixed_interval, &h.calendar_interval) {
        (Some(_), Some(_)) => Err(Error::BadRequest(
            "date_histogram accepts either fixed_interval or calendar_interval, not both".into(),
        )),
        (None, None) => Err(Error::BadRequest(
            "date_histogram requires fixed_interval or calendar_interval".into(),
        )),
        (None, Some(calendar)) => match calendar.parse::<Unit>() {
            Ok(unit) if calendar.len() > 1 && !matches!(unit, Unit::Millisecond | Unit::Second) => {
                Ok(Interval::Calendar(unit))
            }
            _ => Err(Error::BadRequest(format!(
                "invalid calendar_interval [{calendar}]"
            ))),
        },
        (Some(fixed), None) => {
            let invalid = || Error::BadRequest(format!("invalid fixed_interval [{fixed}]"));
            if h.time_zone.fixed_offset_millis().is_none() {
                return Err(Error::BadRequest(
                    "date_histogram with fixed_interval supports only fixed-offset time_zone"
                        .into(),
                ));
            }
            let split = fixed
                .find(|c: char| !c.is_ascii_digit())
                .ok_or_else(invalid)?;
            let (n, unit) = fixed.split_at(split);
            let millis = match unit {
                "ms" => 1,
                "s" => 1_000,
                "m" => 60_000,
                "h" => 3_600_000,
                "d" => 86_400_000,
                _ => return Err(invalid()),
            };
            n.parse::<i64>()
                .ok()
                .and_then(|n| n.checked_mul(millis))
                .filter(|m| *m > 0)
                .map(Interval::Fixed)
                .ok_or_else(invalid)
        }
    }
}

impl Interval {
    fn floor(&self, at: i64, zone: &Zone) -> Option<i64> {
        match self {
            Interval::Fixed(width) => {
                let offset = zone.fixed_offset_millis()?;
                Some((at + offset).div_euclid(*width) * width - offset)
            }
            Interval::Calendar(unit) => {
                let local = zone.local(DateTime::from_timestamp_millis(at)?);
                zone.utc(date::round_down(local, *unit).ok()?)
                    .map(|at| at.timestamp_millis())
            }
        }
    }

    fn next(&self, start: i64, zone: &Zone) -> Option<i64> {
        match self {
            Interval::Fixed(width) => start.checked_add(*width),
            Interval::Calendar(unit) => {
                let local = zone.local(DateTime::from_timestamp_millis(start)?);
                zone.utc(date::add(local, *unit, 1).ok()?)
                    .map(|at| at.timestamp_millis())
            }
        }
    }
}

fn histogram(
    schema: &Schema,
    clause: &AggClause,
    h: &DateHistogramBody,
    docs: Vec<Document>,
) -> Result<AggResult, Error> {
    let interval = interval(h)?;
    let zone = &h.time_zone;
    let min_doc_count = h.min_doc_count.unwrap_or(0);

    let mut merged: BTreeMap<i64, Document> = docs
        .into_iter()
        .filter_map(|mut doc| Some((doc.fields.remove("key")?.as_timestamp()?, doc)))
        .collect();

    if min_doc_count == 0 {
        let edge = |bound: Option<&JsonValue>| match bound {
            None => Ok(None),
            Some(b) => {
                date::millis(b.clone().into_inner(), zone).map(|at| interval.floor(at, zone))
            }
        };
        let bounds = h.extended_bounds.as_ref();
        let min = edge(bounds.and_then(|b| b.min.as_ref()))?;
        let max = edge(bounds.and_then(|b| b.max.as_ref()))?;
        let first = merged.first_key_value().map(|(k, _)| *k);
        let last = merged.last_key_value().map(|(k, _)| *k);
        let lo = [first, min].into_iter().flatten().min();
        let hi = [last, max].into_iter().flatten().max();

        if let Some((mut at, hi)) = lo.or(hi).zip(hi.or(lo)) {
            while at <= hi {
                if merged.len() >= MAX_BUCKETS {
                    return Err(Error::BadRequest(format!(
                        "date_histogram would produce more than {MAX_BUCKETS} buckets"
                    )));
                }
                merged.entry(at).or_default();
                match interval.next(at, zone) {
                    Some(next) => at = next,
                    None => break,
                }
            }
        }
    }

    let mut buckets = merged
        .into_iter()
        .map(|(start, doc)| {
            let key_as_string = date::format(start, zone).ok_or_else(|| {
                Error::BadRequest(format!("date_histogram bucket [{start}] is out of range"))
            })?;
            let key = JsonValue::from(Value::i64(start));
            Ok(bucket(schema, clause, key, Some(key_as_string), doc))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    buckets.retain(|b| b.doc_count >= min_doc_count);

    let buckets = match h.keyed {
        false => Buckets::List(buckets),
        true => Buckets::Keyed(
            buckets
                .into_iter()
                .map(|b| (b.key_as_string.clone().unwrap_or_default(), b))
                .collect(),
        ),
    };
    Ok(AggResult::Histogram { buckets })
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
            AggType::Cardinality(m) => Ok(AggregateExpr::count_distinct(m.field)),
            AggType::Terms(_) | AggType::DateHistogram(_) => Err(Error::Unsupported(
                "Nested bucket sub-aggregations are not supported".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[case(1000, 0, 0)]
    #[case(1000, 999, 0)]
    #[case(1000, 1000, 1000)]
    #[case(1000, -1, -1000)]
    #[case(1000, -1000, -1000)]
    #[case(1000, -1001, -2000)]
    #[case(86_400_000, -1, -86_400_000)]
    fn fixed_floor_walks_down_below_epoch(
        #[case] width: i64,
        #[case] at: i64,
        #[case] expected: i64,
    ) {
        assert_eq!(
            Interval::Fixed(width).floor(at, &Zone::default()),
            Some(expected)
        );
    }
}
