use std::collections::HashMap;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{serde_as, OneOrMany};
use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::data::Value as TopkValue;
use topk_rs::query::SortOrder as TopkSortOrder;

use super::aggs::{AggClause, AggResult};
use super::query::{BoolQuery, FieldClause, FieldName, GateQuery, Query, RangeBounds, TermValue};
use super::source::SourceFilter;
use super::{DocId, IndexName, Shards, Source};
use crate::vector::ensure_finite;
use crate::Error;

pub const MAX_SORT_FIELDS: usize = 8;

// ES's relevance pseudo-field. Sorts on the computed score, not a document field.
pub const SORT_SCORE: &str = "_score";

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

    #[serde(default)]
    pub pit: Option<super::PitRef>,

    #[serde(default)]
    pub search_after: Option<Vec<JsonValue>>,

    // Accepted, not honoured: `hits.total` is already the exact count of returned hits.
    #[serde(default)]
    #[allow(dead_code)]
    pub track_total_hits: Option<JsonValue>,

    // Optimistic-concurrency request flags. Accepted; we don't emit `_seq_no`/`_primary_term`.
    #[serde(default)]
    #[allow(dead_code)]
    pub seq_no_primary_term: Option<bool>,

    #[serde(default)]
    #[allow(dead_code)]
    pub version: Option<bool>,

    // Painless runtime-field scripts (task manager's task-cost calc). Accepted so the query
    // parses; the field it defines does not exist, so aggregations over it return empty. TopK has
    // no Painless — task manager cannot be fully healthy through the shim. See ELASTIC.md.
    #[serde(default)]
    #[allow(dead_code)]
    pub runtime_mappings: Option<serde_json::Value>,

    #[serde(default)]
    #[allow(dead_code)]
    pub script_fields: Option<serde_json::Value>,
}

fn default_size() -> u64 {
    10
}

impl<'de> Deserialize<'de> for SearchRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut req = Self::deserialize(deserializer)?;

        if req.from + req.size > 10_000 {
            return Err(serde::de::Error::custom(format!(
                "Result window is too large, from + size must be less than or equal to 10,000 but was {}",
                req.from + req.size
            )));
        }

        // ES treats an empty sort array as no sort; keep `Some` ⇒ non-empty.
        if req.sort.as_ref().is_some_and(|s| s.is_empty()) {
            req.sort = None;
        }

        req.lower_pit().map_err(serde::de::Error::custom)?;

        Ok(req)
    }
}

pub const SORT_SHARD_DOC: &str = "_shard_doc";
pub const FIELD_ID: &str = "_id";

impl SearchRequest {
    // `_pit` is cursor pagination, not a snapshot (see ELASTIC.md). Kibana asks for ES's internal
    // `_shard_doc` order, which we serve as `_id` ascending — any stable total order works — and
    // `search_after` becomes the lexicographic "resume after this key" predicate: for sort
    // [a, b, c] and cursor [x, y, z] that is
    //   a>x OR (a=x AND b>y) OR (a=x AND b=y AND c>z)
    // with each comparison flipped for a descending key.
    fn lower_pit(&mut self) -> Result<(), Error> {
        if let Some(sort) = self.sort.as_mut() {
            sort.rewrite_shard_doc();
        }

        let cursor = match self.search_after.take() {
            Some(cursor) => cursor,
            None => return Ok(()),
        };

        let sort = self
            .sort
            .as_ref()
            .ok_or_else(|| Error::BadRequest("\"search_after\" requires a \"sort\"".into()))?;

        if sort.len() != cursor.len() {
            return Err(Error::BadRequest(format!(
                "\"search_after\" has {} value(s) but \"sort\" has {} key(s)",
                cursor.len(),
                sort.len()
            )));
        }

        let mut alternatives = Vec::with_capacity(sort.len());
        for (i, key) in sort.iter().enumerate() {
            let field = match &key.target {
                SortTarget::Field(field) => field.clone(),
                SortTarget::Score => {
                    return Err(Error::Unsupported(
                        "\"search_after\" over a \"_score\" sort is not supported".into(),
                    ))
                }
            };

            let mut clauses = Vec::with_capacity(i + 1);
            for (prior, value) in sort.iter().zip(&cursor).take(i) {
                if let SortTarget::Field(prior) = &prior.target {
                    clauses.push(Query::Term(FieldClause {
                        field: prior.clone(),
                        value: TermValue::Bare(value.clone()),
                    }));
                }
            }

            let bound = cursor[i].clone();
            clauses.push(Query::Range(FieldClause {
                field,
                value: match key.asc {
                    true => RangeBounds {
                        gt: Some(bound),
                        ..Default::default()
                    },
                    false => RangeBounds {
                        lt: Some(bound),
                        ..Default::default()
                    },
                },
            }));

            alternatives.push(match clauses.len() {
                1 => clauses.remove(0),
                _ => Query::Bool(BoolQuery {
                    must: clauses,
                    ..Default::default()
                }),
            });
        }

        let resume = Query::Bool(BoolQuery {
            should: alternatives,
            ..Default::default()
        });

        self.query = Some(match self.query.take() {
            Some(query) => Query::Bool(BoolQuery {
                must: vec![query, resume],
                ..Default::default()
            }),
            None => resume,
        });

        Ok(())
    }
}

impl SortClause {
    fn rewrite_shard_doc(&mut self) {
        for sort in self.0.iter_mut() {
            if sort.target.is_field(SORT_SHARD_DOC) {
                sort.target = SortTarget::Field(FieldName::new(FIELD_ID));
            }
        }
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

    #[serde(default, alias = "window_size")]
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

impl QueryVector {
    fn value(&self) -> &TopkValue {
        match self {
            QueryVector::Flat(value) | QueryVector::Matrix(value) => value,
        }
    }
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

        let vector = match wire {
            QueryVectorWire::Flat(numbers) => {
                TopkValue::try_from(values(numbers)).map(QueryVector::Flat)
            }
            QueryVectorWire::Matrix(rows) => {
                TopkValue::try_from(rows.into_iter().map(values).collect::<Vec<_>>())
                    .map(QueryVector::Matrix)
            }
        }
        .map_err(|e| Error::InvalidQuery(e.to_string()))?;

        ensure_finite(vector.value())?;

        Ok(vector)
    }
}

#[derive(Deserialize)]
#[serde(try_from = "SortWire")]
pub struct SortClause(Vec<SortField>);

impl Deref for SortClause {
    type Target = [SortField];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct SortField {
    pub target: SortTarget,
    pub asc: bool,
}

impl SortField {
    pub fn is_score(&self) -> bool {
        self.target.is_score()
    }

    pub fn field_name(&self) -> Option<&FieldName> {
        match &self.target {
            SortTarget::Score => None,
            SortTarget::Field(name) => Some(name),
        }
    }

    pub fn order(&self) -> TopkSortOrder {
        match self.asc {
            true => TopkSortOrder::Asc,
            false => TopkSortOrder::Desc,
        }
    }
}

pub enum SortTarget {
    Score,
    Field(FieldName),
}

impl SortTarget {
    pub fn is_score(&self) -> bool {
        matches!(self, SortTarget::Score)
    }

    pub fn is_field(&self, name: &str) -> bool {
        matches!(self, SortTarget::Field(f) if f.as_str() == name)
    }
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

        if sorts.len() > MAX_SORT_FIELDS {
            return Err(Error::Unsupported(format!(
                "Sort supports at most {MAX_SORT_FIELDS} fields but [{}] were given",
                sorts.len()
            )));
        }

        let fields = sorts
            .into_iter()
            .map(|sort| {
                let target = match sort.name().as_str() {
                    SORT_SCORE => SortTarget::Score,
                    _ => SortTarget::Field(sort.name().clone()),
                };

                SortField {
                    // Without an explicit order, `_score` sorts descending in ES
                    // and every other field ascending.
                    asc: match sort.order() {
                        Some(order) => matches!(order, SortOrder::Asc),
                        None => !target.is_score(),
                    },
                    target,
                }
            })
            .collect();

        Ok(SortClause(fields))
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

    fn order(&self) -> Option<&SortOrder> {
        match self {
            Sort::Bare(_) => None,
            Sort::Field(clause) => clause.value.order(),
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

    // Docs missing a sort field already sort last in both directions, so `_last`
    // is a no-op. `_first` would change the result, so it stays rejected.
    #[serde(default)]
    #[allow(dead_code)]
    missing: Option<Missing>,
}

#[derive(Deserialize)]
enum Missing {
    #[serde(rename = "_last")]
    Last,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_id: Option<String>,
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
            pit_id: None,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<JsonValue>>,
    #[serde(rename = "_source", skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}
