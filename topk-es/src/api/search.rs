use std::collections::HashMap;
use std::ops::Deref;

use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, OneOrMany};
use topk_rs::json::Value as JsonValue;
use topk_rs::proto::v1::data::{list, value, Value as TopkValue};
use topk_rs::query::SortOrder as TopkSortOrder;

use super::aggs::{AggClause, AggResult};
use super::query::{FieldClause, FieldName, GateQuery, Query};
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

    #[serde(default, alias = "window_size")]
    pub rank_window_size: Option<u64>,
}

// Query vectors are parsed like document values (whole numbers stay integers)
// so the engine can coerce them to the target field's element type.
#[derive(Clone)]
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

impl<'de> Deserialize<'de> for QueryVector {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = JsonValue::deserialize(deserializer)?.into_inner();

        let vector =
            match &value.value {
                // A matrix is always numeric; a list has to be checked.
                Some(value::Value::List(list))
                    if !matches!(list.values, Some(list::Values::String(_))) =>
                {
                    QueryVector::Flat(value)
                }
                Some(value::Value::Matrix(_)) => QueryVector::Matrix(value),
                _ => return Err(serde::de::Error::custom(
                    "[knn] query_vector must be an array of numbers, or an array of such arrays",
                )),
            };

        ensure_finite(vector.value()).map_err(serde::de::Error::custom)?;

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
}

impl SearchResponse {
    pub fn new(
        index: &IndexName,
        hits: Vec<Hit>,
        aggregations: Option<HashMap<String, AggResult>>,
        matched: &[u64],
    ) -> Self {
        let max_score = hits.iter().filter_map(|h| h.score).reduce(f32::max);
        Self {
            took: 1,
            timed_out: false,
            shards: Shards::default(),
            hits: HitsWrapper {
                total: match matched {
                    // No matched counts reported, so hits are only a lower bound
                    [] => Total {
                        value: hits.len() as u64,
                        relation: "gte",
                    },
                    // Single retriever
                    [matched] => Total {
                        value: *matched,
                        relation: "eq",
                    },
                    // Multiple retrievers
                    m => Total {
                        value: m.iter().copied().max().unwrap().max(hits.len() as u64),
                        relation: "gte",
                    },
                },
                max_score,
                index: index.clone(),
                hits,
            },
            aggregations,
        }
    }
}

// Every hit reports the same `_index`, so it is held once and written into each
// hit on the way out rather than cloned per hit.
pub struct HitsWrapper {
    pub total: Total,
    pub max_score: Option<f32>,
    pub index: IndexName,
    pub hits: Vec<Hit>,
}

impl Serialize for HitsWrapper {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("total", &self.total)?;
        map.serialize_entry("max_score", &self.max_score)?;
        map.serialize_entry(
            "hits",
            &IndexedHits {
                index: &self.index,
                hits: &self.hits,
            },
        )?;
        map.end()
    }
}

struct IndexedHits<'a> {
    index: &'a IndexName,
    hits: &'a [Hit],
}

impl Serialize for IndexedHits<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.hits.iter().map(|hit| IndexedHit {
            index: self.index,
            hit,
        }))
    }
}

#[derive(Serialize)]
struct IndexedHit<'a> {
    #[serde(rename = "_index")]
    index: &'a IndexName,
    #[serde(flatten)]
    hit: &'a Hit,
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // Whole numbers stay integers so the engine can narrow them to the target
    // field's element type.
    #[rstest]
    #[case::floats("[1.0, 0.0]", TopkValue::list(vec![1.0_f32, 0.0]))]
    #[case::ints("[127, 0]", TopkValue::list(vec![127_i64, 0]))]
    #[case::signed_ints("[-1, 1]", TopkValue::list(vec![-1_i64, 1]))]
    fn flat_query_vector(#[case] body: &str, #[case] expected: TopkValue) {
        let vector: QueryVector = sonic_rs::from_str(body).unwrap();

        assert!(matches!(vector, QueryVector::Flat(_)));
        assert_eq!(vector.value(), &expected);
    }

    #[test]
    fn matrix_query_vector() {
        let vector: QueryVector = sonic_rs::from_str("[[1.0, 0.0], [0.5, 0.5]]").unwrap();

        assert!(matches!(vector, QueryVector::Matrix(_)));
        assert_eq!(
            vector.value(),
            &TopkValue::matrix(2, vec![1.0_f32, 0.0, 0.5, 0.5])
        );
    }

    #[rstest]
    #[case::scalar("1.0")]
    #[case::strings(r#"["a"]"#)]
    #[case::object(r#"{"0": 1.0}"#)]
    #[case::ragged("[[1.0], [1.0, 2.0]]")]
    // A number past f32's range lands as infinity, which ES rejects.
    #[case::overflow("[1e39, 0.0]")]
    fn query_vector_rejected(#[case] body: &str) {
        assert!(sonic_rs::from_str::<QueryVector>(body).is_err());
    }

    // A whole request, to keep the derive-heavy corners — externally tagged
    // query clauses, `OneOrMany`, flattened agg types — honest.
    #[rstest]
    #[case::single(
        r#"{
            "query": {"bool": {"must": {"match": {"title": "a"}}, "filter": {"term": {"genre": "b"}}}},
            "knn": {"field": "embedding", "query_vector": [1.0, 0.0], "k": 2},
            "sort": {"year": "desc"}
        }"#
    )]
    #[case::many(
        r#"{
            "query": {"bool": {"must": [{"match": {"title": "a"}}], "filter": [{"term": {"genre": "b"}}]}},
            "knn": [{"field": "embedding", "query_vector": [1.0, 0.0], "k": 2}],
            "sort": [{"year": "desc"}]
        }"#
    )]
    fn accepts_one_or_many(#[case] body: &str) {
        let req: SearchRequest = sonic_rs::from_str(body).unwrap();

        assert_eq!(req.knn.as_ref().map(Vec::len), Some(1));
        assert_eq!(req.sort.as_deref().map(<[SortField]>::len), Some(1));
        assert!(matches!(req.query, Some(Query::Bool(_))));
        assert_eq!(req.size, 10, "size defaults to 10");
    }

    #[test]
    fn reads_aggs_and_source_filters() {
        let req: SearchRequest = sonic_rs::from_str(
            r#"{
                "size": 5,
                "from": 1,
                "track_scores": true,
                "aggs": {
                    "by_genre": {"terms": {"field": "genre", "size": 3}},
                    "total": {"sum": {"field": "year"}}
                },
                "_source": {"includes": ["title"]}
            }"#,
        )
        .unwrap();

        assert_eq!((req.size, req.from), (5, 1));
        assert!(req.track_scores);
        assert_eq!(req.aggs.len(), 2);
        assert!(req.source.enabled());
        assert!(req.source.keep("title") && !req.source.keep("genre"));
    }

    #[test]
    fn empty_sort_is_no_sort() {
        let req: SearchRequest = sonic_rs::from_str(r#"{"sort": []}"#).unwrap();
        assert!(req.sort.is_none());
    }

    #[rstest]
    #[case::window_too_large(r#"{"from": 9999, "size": 10}"#)]
    #[case::unknown_field(r#"{"nope": 1}"#)]
    #[case::knn_without_k(r#"{"knn": {"field": "e", "query_vector": [1.0]}}"#)]
    #[case::zero_k(r#"{"knn": {"field": "e", "query_vector": [1.0], "k": 0}}"#)]
    #[case::too_few_candidates(
        r#"{"knn": {"field": "e", "query_vector": [1.0], "k": 2, "num_candidates": 1}}"#
    )]
    fn search_request_rejected(#[case] body: &str) {
        assert!(sonic_rs::from_str::<SearchRequest>(body).is_err());
    }
}
