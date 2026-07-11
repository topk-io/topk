use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{serde_as, OneOrMany};
use topk_rs::proto::v1::data::Value as TopkValue;

use super::aggs::{AggClause, AggResult};
use super::query::{FieldClause, FieldName, GateQuery, Query};
use super::source::SourceFilter;
use super::{DocId, IndexName, Shards, Source};
use crate::Error;

#[serde_as]
#[derive(Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct SearchRequest {
    #[serde(default)]
    pub query: Option<Query>,

    #[serde(default = "default_size")]
    pub size: u64,

    #[serde(default)]
    pub from: u64,

    #[serde(default)]
    pub sort: Option<SortClause>,

    #[serde_as(as = "Option<OneOrMany<_>>")]
    #[serde(default)]
    pub knn: Option<Vec<KnnRequest>>,

    #[serde(default)]
    pub rank: Option<RankClause>,

    #[serde(default)]
    pub track_scores: bool,

    #[serde(default, alias = "aggregations")]
    pub aggs: HashMap<String, AggClause>,

    #[serde(default, rename = "_source")]
    pub source: SourceFilter,
}

fn default_size() -> u64 {
    10
}

impl<'de> Deserialize<'de> for SearchRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let req = Self::deserialize(deserializer)?;
        if req.from + req.size > 10_000 {
            return Err(serde::de::Error::custom(format!(
                "Result window is too large, from + size must be less than or equal to 10,000 but was {}",
                req.from + req.size
            )));
        }
        Ok(req)
    }
}

#[serde_as]
#[derive(Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct KnnRequest {
    pub field: FieldName,
    pub query_vector: QueryVector,
    pub k: u64,

    #[serde_as(as = "OneOrMany<_>")]
    #[serde(default)]
    pub filter: Vec<GateQuery>,

    #[serde(default)]
    pub num_candidates: Option<u64>,

    #[serde(default)]
    pub boost: Option<f32>,

    #[serde(default)]
    pub similarity: Option<f32>,
}

impl<'de> Deserialize<'de> for KnnRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let req = Self::deserialize(deserializer)?;
        if req.k == 0 {
            return Err(serde::de::Error::custom("[knn] k must be greater than 0"));
        }
        if let Some(candidates) = req.num_candidates {
            if candidates < req.k {
                return Err(serde::de::Error::custom(format!(
                    "\"num_candidates\" ({candidates}) cannot be less than k ({})",
                    req.k
                )));
            }
        }
        Ok(req)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankClause {
    pub rrf: RrfClause,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrfClause {
    #[serde(default)]
    pub rank_constant: Option<f32>,

    #[serde(default)]
    pub rank_window_size: Option<u64>,
}

// Query vectors are parsed like document values (whole numbers stay integers)
// so the engine can coerce them to the target field's element type.
#[derive(Clone, Deserialize)]
#[serde(try_from = "QueryVectorWire")]
pub enum QueryVector {
    Flat(TopkValue),
    Matrix(TopkValue),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum QueryVectorWire {
    Flat(Vec<serde_json::Number>),
    Matrix(Vec<Vec<serde_json::Number>>),
}

impl TryFrom<QueryVectorWire> for QueryVector {
    type Error = Error;

    fn try_from(wire: QueryVectorWire) -> Result<Self, Self::Error> {
        let values = |numbers: Vec<serde_json::Number>| {
            numbers
                .into_iter()
                .map(serde_json::Value::Number)
                .collect::<Vec<_>>()
        };

        match wire {
            QueryVectorWire::Flat(numbers) => TopkValue::try_from(values(numbers))
                .map(QueryVector::Flat)
                .map_err(|e| Error::InvalidQuery(e.to_string())),
            QueryVectorWire::Matrix(rows) => {
                TopkValue::try_from(rows.into_iter().map(values).collect::<Vec<_>>())
                    .map(QueryVector::Matrix)
                    .map_err(|e| Error::InvalidQuery(e.to_string()))
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(try_from = "SortWire")]
pub struct SortClause {
    pub field: FieldName,
    pub asc: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SortWire {
    One(Sort),
    Many(Vec<Sort>),
}

impl TryFrom<SortWire> for SortClause {
    type Error = Error;

    fn try_from(wire: SortWire) -> Result<Self, Self::Error> {
        let sorts = match wire {
            SortWire::One(sort) => vec![sort],
            SortWire::Many(sorts) => sorts,
        };
        if sorts.len() > 1 {
            return Err(Error::Unsupported(
                "Multi-field sort is not supported".into(),
            ));
        }
        let sort = sorts
            .into_iter()
            .next()
            .ok_or_else(|| Error::InvalidQuery("\"sort\" must not be empty".into()))?;
        Ok(SortClause {
            field: sort.name().clone(),
            asc: matches!(sort.order(), SortOrder::Asc),
        })
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Sort {
    Bare(FieldName),
    Field(FieldClause<SortValue>),
}

impl Sort {
    fn name(&self) -> &FieldName {
        match self {
            Sort::Bare(name) => name,
            Sort::Field(clause) => &clause.field,
        }
    }

    fn order(&self) -> &SortOrder {
        match self {
            Sort::Bare(_) => &SortOrder::Asc,
            Sort::Field(clause) => clause.value.order().unwrap_or(&SortOrder::Asc),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SortValue {
    Order(SortOrder),
    Full(SortValueFull),
}

impl SortValue {
    fn order(&self) -> Option<&SortOrder> {
        match self {
            SortValue::Order(order) => Some(order),
            SortValue::Full(full) => full.order.as_ref(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SortValueFull {
    #[serde(default)]
    order: Option<SortOrder>,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SortOrder {
    #[default]
    #[serde(alias = "ASC")]
    Asc,
    #[serde(alias = "DESC")]
    Desc,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub took: u32,
    pub timed_out: bool,
    #[serde(rename = "_shards")]
    pub shards: Shards,
    pub hits: HitsWrapper,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<HashMap<String, AggResult>>,
}

impl SearchResponse {
    pub fn new(
        index: &IndexName,
        hits: Vec<Hit>,
        aggregations: Option<HashMap<String, AggResult>>,
    ) -> Self {
        let max_score = hits.iter().filter_map(|h| h.score).reduce(f32::max);
        Self {
            took: 1,
            timed_out: false,
            shards: Shards::default(),
            hits: HitsWrapper {
                total: Total {
                    value: hits.len() as u64,
                    relation: "eq",
                },
                max_score,
                hits: hits
                    .into_iter()
                    .map(|hit| IndexedHit {
                        index: index.clone(),
                        hit,
                    })
                    .collect(),
            },
            aggregations,
        }
    }
}

#[derive(Serialize)]
pub struct HitsWrapper {
    pub total: Total,
    pub max_score: Option<f32>,
    pub hits: Vec<IndexedHit>,
}

#[derive(Serialize)]
pub struct IndexedHit {
    #[serde(rename = "_index")]
    pub index: IndexName,
    #[serde(flatten)]
    pub hit: Hit,
}

#[derive(Serialize)]
pub struct Total {
    pub value: u64,
    pub relation: &'static str,
}

#[derive(Serialize)]
pub struct Hit {
    #[serde(rename = "_id")]
    pub id: DocId,
    #[serde(rename = "_score")]
    pub score: Option<f32>,
    #[serde(rename = "_source", skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

#[cfg(test)]
mod tests {
    use super::KnnRequest;

    #[test]
    fn knn_k_zero_is_rejected() {
        assert!(
            serde_json::from_str::<KnnRequest>(r#"{"field":"vec","query_vector":[1,0],"k":0}"#)
                .is_err()
        );
    }
}
