use std::collections::HashMap;

use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::data::{AggregateExpr, Document, LogicalExpr, Query as TopkQuery, Value};
use topk_rs::query::{field, filter};

use super::value::ValueExt;
use crate::api::{AggClause, AggResult, AggType, TermsBucket};
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
                .sort(field("doc_count"), false)
                .limit(terms.size.unwrap_or(10) as u64);

            Ok(query)
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

pub fn collect(clause: &AggClause, docs: Vec<Document>) -> Result<AggResult, Error> {
    match &clause.ty {
        AggType::Terms(_) => {
            let mut buckets = Vec::with_capacity(docs.len());

            for mut doc in docs {
                let key = doc.fields.remove("key").unwrap_or_else(Value::null);

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
                    key: JsonValue::from(key),
                    doc_count,
                    sub_aggs,
                });
            }

            Ok(AggResult::Terms {
                doc_count_error_upper_bound: 0,
                sum_other_doc_count: 0,
                buckets,
            })
        }
        _ => {
            let value = docs
                .into_iter()
                .next()
                .and_then(|mut doc| doc.fields.remove("value"))
                .and_then(|v| v.number());

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
            AggType::Terms(_) => Err(Error::Unsupported(
                "Nested \"terms\" sub-aggregations are not supported".into(),
            )),
        }
    }
}
